//! The layout: layout lines and logical lines, columns and indexes, the width of a string, the
//! size of the view, and the tab width and the wrap column.
//!
//! Sakura counts a *layout line* with wrapping (`CLayout`, one per row on the screen) and a
//! *logical line* without (`CDocLine`, up to a line break). A *layout column* is in cells; a
//! *logical index* is in `wchar_t` (UTF-16 code units). The functions of this group take and
//! return all of them from 1. This editor's layout rows are Sakura's layout lines
//! ([`ume_commands::layout::Layout`]).
//!
//! Differences from Sakura, where a position is outside the text:
//!
//! * A layout column past the end of a row gives the end of the text of the row. Sakura's loop
//!   (`CLayoutMgr::LayoutToLogicEx`) runs on over the line ending and gives an index after it.
//! * A logical index after the end of a line that has a line ending gives the column after the
//!   text plus one (the room for the line ending mark), as Sakura does at the end of a line
//!   (`CLayoutMgr::LogicToLayout`); the positions inside a two-character line ending are not told
//!   apart.

use serde_json::Value;
use ume_commands::context::CommandContext;
use ume_core::buffer::Pos;

use super::{EditorFunc, command, unavailable, utf16_len};
use crate::api::{Args, Ty, byte_offset_for_utf16, opt, req};
use crate::host::Editor;
use crate::protocol::RpcError;

type Result<T = Value> = std::result::Result<T, RpcError>;

/// `MAXLINEKETAS` and `MINLINEKETAS` (`sakura_core/config/app_constants.h`): the widest and the
/// narrowest wrap column.
const MAX_LINE_COLUMNS: i64 = 10240;
const MIN_LINE_COLUMNS: i64 = 10;
/// The widest tab that `ChangeTabWidth` accepts (the help of `S_ChangeTabWidth`: 1 to 64).
const MAX_TAB_WIDTH: i64 = 64;

/// The layout row of `pos` and its display column in the row, both counted from 0.
pub(super) fn layout_point(cx: &CommandContext, pos: Pos) -> Result<(usize, usize)> {
    let row = cx.layout().row_index(cx.doc(), pos).map_err(command)?;
    let col = cx.layout().display_col(cx.doc(), pos).map_err(command)?;
    Ok((row, col))
}

/// The number of logical lines: the position after the last line break is not a line, and an
/// empty document has none (`CDocLineMgr::GetLineCount`, which is `Document::line_count`).
fn logical_line_count(cx: &CommandContext) -> usize {
    cx.doc().line_count()
}

/// Whether the document ends with an empty row of its own: the position after a final line break,
/// or an empty document. It is a row but not a line.
fn has_empty_last_row(cx: &CommandContext) -> bool {
    cx.doc().row_count() > cx.doc().line_count()
}

/// The number of layout lines (`CLayoutMgr::GetLineCount`): the rows, without the empty row after
/// a final line break.
fn layout_line_count(cx: &CommandContext) -> Result<usize> {
    let doc = cx.doc();
    let end = doc.end_pos().map_err(command)?;
    let rows = cx.layout().row_index(doc, end).map_err(command)? + 1;
    Ok(rows - usize::from(has_empty_last_row(cx)))
}

/// `CLayoutMgr::LogicToLayout`: the layout line and column (from 0) of the index `index16` (in
/// UTF-16 units, from 0) of the logical line `line`. A line past the last one is the line after
/// the last layout line, column 0.
fn logic_to_layout(cx: &CommandContext, line: usize, index16: i64) -> Result<(usize, usize)> {
    if line >= logical_line_count(cx) {
        return Ok((layout_line_count(cx)?, 0));
    }
    let text = cx
        .doc()
        .line(line)
        .map_err(command)?
        .map(|l| l.text)
        .unwrap_or_default();
    let units = usize::try_from(index16).unwrap_or(0);
    if units > utf16_len(&text) {
        // After the end: the column after the text, and the line ending mark if there is one.
        let end = Pos::new(line, text.len());
        let (row, col) = layout_point(cx, end)?;
        let has_break = line + 1 < cx.doc().row_count();
        return Ok((row, col + usize::from(has_break)));
    }
    layout_point(cx, Pos::new(line, byte_offset_for_utf16(&text, units)))
}

/// `CLayoutMgr::LayoutToLogicEx`: the logical line and the index (UTF-16 units) of the layout
/// line `row` and column `col`, both from 0 (`col` may be negative, which counts as 0).
fn layout_to_logic(cx: &CommandContext, row: usize, col: i64) -> Result<(usize, usize)> {
    let lines = layout_line_count(cx)?;
    if row > lines {
        return Ok((logical_line_count(cx), 0));
    }
    if row == lines {
        // The row after the last one: the empty line after a final line break is the start of
        // nothing; after a last line without a line break it is the end of that line.
        let logical = logical_line_count(cx);
        if has_empty_last_row(cx) {
            return Ok((logical, 0));
        }
        let last = logical - 1;
        let text = cx
            .doc()
            .line(last)
            .map_err(command)?
            .map(|l| l.text)
            .unwrap_or_default();
        return Ok((last, utf16_len(&text)));
    }
    let start = cx
        .layout()
        .row_start_at(cx.doc(), row)
        .map_err(command)?
        .ok_or_else(|| RpcError::invalid_params("there is no such layout line"))?;
    let col = usize::try_from(col).unwrap_or(0);
    let pos = cx
        .layout()
        .pos_at_col(cx.doc(), start, col)
        .map_err(command)?;
    let text = cx
        .doc()
        .line(pos.line)
        .map_err(command)?
        .map(|l| l.text)
        .unwrap_or_default();
    Ok((pos.line, utf16_len(text.get(..pos.col).unwrap_or(&text))))
}

/// F_LAYOUTTOLOGICLINENUM: the logical line of a layout line. The line after the last layout line
/// gives the line after the last logical line; any other line outside the text is an error.
fn layout_to_logic_line_num(ed: &mut Editor, args: &Args) -> Result {
    let row = args.int(0).saturating_sub(1);
    let cx = &*ed.cx;
    let lines = layout_line_count(cx)?;
    if row == i64::try_from(lines).unwrap_or(i64::MAX) {
        return Ok((logical_line_count(cx) as i64 + 1).into());
    }
    let row = usize::try_from(row)
        .ok()
        .filter(|&r| r < lines)
        .ok_or_else(|| {
            RpcError::invalid_params("LayoutToLogicLineNum: there is no such layout line")
        })?;
    let start = cx
        .layout()
        .row_start_at(cx.doc(), row)
        .map_err(command)?
        .ok_or_else(|| {
            RpcError::invalid_params("LayoutToLogicLineNum: there is no such layout line")
        })?;
    Ok((start.line as i64 + 1).into())
}

/// F_LINECOLUMNTOINDEX: the logical index (from 1, in UTF-16 units) of a layout line and column.
/// The line must be 1 or more; the column is not checked.
fn line_column_to_index(ed: &mut Editor, args: &Args) -> Result {
    let (line, col) = (args.int(0).saturating_sub(1), args.int(1).saturating_sub(1));
    let row = usize::try_from(line).map_err(|_| {
        RpcError::invalid_params("LineColumnToIndex: the layout line must be 1 or more")
    })?;
    let (_, index) = layout_to_logic(&*ed.cx, row, col)?;
    Ok((index as i64 + 1).into())
}

/// F_LOGICTOLAYOUTLINENUM and F_LINEINDEXTOCOLUMN: the layout line, or the layout column, of a
/// logical line and index. Both start from 1. The line must be 1 or more; the index is not
/// checked.
fn logic_to_layout_call(ed: &mut Editor, args: &Args, column: bool) -> Result {
    let (line, index) = (args.int(0).saturating_sub(1), args.int(1).saturating_sub(1));
    let line = usize::try_from(line)
        .map_err(|_| RpcError::invalid_params("the logical line must be 1 or more"))?;
    let (row, col) = logic_to_layout(&*ed.cx, line, index)?;
    Ok((if column { col } else { row } as i64 + 1).into())
}

fn logic_to_layout_line_num(ed: &mut Editor, args: &Args) -> Result {
    logic_to_layout_call(ed, args, false)
}

fn line_index_to_column(ed: &mut Editor, args: &Args) -> Result {
    logic_to_layout_call(ed, args, true)
}

/// The width in cells of `text` that starts at layout column `start` (counted from 0): a tab
/// goes to the next tab stop, the other characters have the width of the layout.
fn text_width(cx: &CommandContext, text: &str, start: i64) -> i64 {
    let tab = i64::try_from(cx.layout().tab_width()).unwrap_or(4).max(1);
    let start = start.max(0);
    let mut x = start;
    for c in text.chars() {
        if c == '\t' {
            x = x.saturating_add(tab - x % tab);
        } else {
            x = x.saturating_add(i64::try_from(cx.layout().char_columns(c)).unwrap_or(1));
        }
    }
    x.saturating_sub(start)
}

/// F_GETSTRWIDTH: the width of a string in cells, from layout column `start` (from 1, default 1;
/// the column matters for tabs). The line break of CR LF at the end counts as the CR alone
/// (`pLine[nLen - 2] == CR && pLine[nLen - 1] == LF`: `nLen--`).
fn get_str_width(ed: &mut Editor, args: &Args) -> Result {
    let text = args.str(0);
    let start = args.int_or(1, 1).saturating_sub(1);
    let text = text
        .strip_suffix("\r\n")
        .map_or(text, |t| &text[..t.len() + 1]);
    Ok(text_width(&*ed.cx, text, start).into())
}

/// F_GETSTRLAYOUTLENGTH: like `GetStrWidth`, but a line ending at the end is part of the string
/// and takes one cell (its mark), whatever it is made of. (Sakura measures the string as a line
/// of the document with `CMemoryIterator`; the width of a line ending there is one cell.)
fn get_str_layout_length(ed: &mut Editor, args: &Args) -> Result {
    let text = args.str(0);
    let start = args.int_or(1, 1).saturating_sub(1);
    let (body, eol) = match text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix(['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}']))
    {
        Some(body) => (body, 1),
        None => (text, 0),
    };
    Ok((text_width(&*ed.cx, body, start) + eol).into())
}

/// F_GETDEFAULTCHARLENGTH: the layout width of a half-width character. Sakura answers in its
/// layout unit (the pitch of the font in pixels, with the character spacing); this editor lays
/// out in whole cells, so a half-width character is 1, and a macro that divides a width by it
/// gets the number of cells.
fn get_default_char_length(_: &mut Editor, _: &Args) -> Result {
    Ok(1.into())
}

fn view(ed: &Editor) -> Result<crate::host::ViewInfo> {
    ed.env
        .view
        .ok_or_else(|| unavailable("the size of the view"))
}

/// F_GETVIEWLINES: how many rows the text area shows (`CTextArea::m_nViewRowNum`).
fn get_view_lines(ed: &mut Editor, _: &Args) -> Result {
    Ok(view(ed)?.rows.into())
}

/// F_GETVIEWCOLUMNS: how many columns the text area shows (`m_nViewColNum`).
fn get_view_columns(ed: &mut Editor, _: &Args) -> Result {
    Ok(view(ed)?.columns.into())
}

/// F_GETVIEWTOP: the layout line at the top of the text area, counted from 1.
fn get_view_top(ed: &mut Editor, _: &Args) -> Result {
    Ok((view(ed)?.top_row + 1).into())
}

/// F_CHGTABWIDTH: answers the tab width, and sets a new one for this view when the argument is
/// 1 to 64 and different. Any other argument only reads (the help: "0 to read"). The change is
/// temporary, as in Sakura (`m_bTabSpaceCurTemp`).
fn change_tab_width(ed: &mut Editor, args: &Args) -> Result {
    let old = i64::try_from(ed.cx.layout().tab_width()).unwrap_or(4);
    let new = args.int(0);
    // Even a width that is the one in use is asked for: the width in use may be the fallback of 4
    // for a tab width that does not fit the wrap column, and a later wrap column would bring the
    // old width back. The window ignores a request for the width it has.
    if (1..=MAX_TAB_WIDTH).contains(&new) {
        ed.env.requests.tab_width = u32::try_from(new).ok();
    }
    Ok(old.into())
}

/// F_CHGWRAPCOLUMN: answers the wrap column, and wraps at the given column for this view when it
/// is 10 to 10240. Any other argument only reads.
fn change_wrap_column(ed: &mut Editor, args: &Args) -> Result {
    let old = ed
        .env
        .wrap_columns
        .ok_or_else(|| unavailable("the wrap column"))?;
    let new = args.int(0);
    if (MIN_LINE_COLUMNS..=MAX_LINE_COLUMNS).contains(&new) {
        ed.env.requests.wrap_columns = u32::try_from(new).ok();
    }
    Ok(i64::from(old).into())
}

/// F_CHGINSSPACE: answers whether the Tab key inserts spaces (`m_bInsSpace`), and sets it to
/// the argument: 0 off, 1 on, anything else only reads. Sakura reads the argument from
/// `VARIANT::iVal`, the low 16 bits of the number, so 65536 is 0 and 65537 is 1
/// (`sakura_core/macro/CMacro.cpp`, `F_CHGINSSPACE`). The change is for this view only (Sakura
/// changes the type settings in memory).
fn change_insert_space(ed: &mut Editor, args: &Args) -> Result {
    let old = ed
        .env
        .ins_space
        .ok_or_else(|| unavailable("the insert-spaces setting"))?;
    match args.int(0) as i16 {
        0 => ed.env.requests.ins_space = Some(false),
        1 => ed.env.requests.ins_space = Some(true),
        _ => {}
    }
    Ok(i64::from(old).into())
}

pub const FUNCS: &[EditorFunc] = &[
    EditorFunc {
        name: "ChangeInsertSpace",
        params: &[req("value", Ty::Int)],
        result: Ty::Int,
        doc: "Whether the Tab key inserts spaces instead of a tab character, before the call \
              (1 or 0). 0 turns it off, 1 turns it on for this window (not saved); any other \
              value only reads. Only the low 16 bits of the value count, as in Sakura.",
        run: change_insert_space,
    },
    EditorFunc {
        name: "LayoutToLogicLineNum",
        params: &[req("layoutLine", Ty::Int)],
        result: Ty::Int,
        doc: "The logical line (up to a line break) that a layout line (a row on the screen) \
              belongs to, both counted from 1. The line after the last layout line gives the \
              line after the last logical line. Any other line outside the text is an error.",
        run: layout_to_logic_line_num,
    },
    EditorFunc {
        name: "LogicToLayoutLineNum",
        params: &[req("logicLine", Ty::Int), req("logicIndex", Ty::Int)],
        result: Ty::Int,
        doc: "The layout line (a row on the screen) of a position in a logical line: the line \
              and the index in it (UTF-16 code units), both counted from 1. A line past the end \
              gives the line after the last layout line.",
        run: logic_to_layout_line_num,
    },
    EditorFunc {
        name: "LineColumnToIndex",
        params: &[req("layoutLine", Ty::Int), req("layoutColumn", Ty::Int)],
        result: Ty::Int,
        doc: "The index (UTF-16 code units, from 1) in the logical line of a layout line and a \
              layout column (cells), both from 1. A column past the text gives the end of the \
              text of the row.",
        run: line_column_to_index,
    },
    EditorFunc {
        name: "LineIndexToColumn",
        params: &[req("logicLine", Ty::Int), req("logicIndex", Ty::Int)],
        result: Ty::Int,
        doc: "The layout column (cells, from 1) of a position in a logical line: the line and \
              the index in it (UTF-16 code units), both from 1. Tabs and full-width characters \
              take their width.",
        run: line_index_to_column,
    },
    EditorFunc {
        name: "GetStrWidth",
        params: &[req("text", Ty::Str), opt("column", Ty::Int)],
        result: Ty::Int,
        doc: "The width of a string in cells, starting at layout column `column` (from 1, \
              default 1; it matters for tabs). A CR LF at the end counts as the CR alone.",
        run: get_str_width,
    },
    EditorFunc {
        name: "GetStrLayoutLength",
        params: &[req("text", Ty::Str), opt("column", Ty::Int)],
        result: Ty::Int,
        doc: "The width of a string as a line of the document in cells, starting at layout \
              column `column` (from 1, default 1). A line ending at the end takes one cell.",
        run: get_str_layout_length,
    },
    EditorFunc {
        name: "GetDefaultCharLength",
        params: &[],
        result: Ty::Int,
        doc: "The layout width of a half-width character. This editor lays out in cells, so it \
              is 1.",
        run: get_default_char_length,
    },
    EditorFunc {
        name: "GetViewLines",
        params: &[],
        result: Ty::Int,
        doc: "How many rows the text area shows (at least 1).",
        run: get_view_lines,
    },
    EditorFunc {
        name: "GetViewColumns",
        params: &[],
        result: Ty::Int,
        doc: "How many columns the text area shows (at least 1).",
        run: get_view_columns,
    },
    EditorFunc {
        name: "GetViewTop",
        params: &[],
        result: Ty::Int,
        doc: "The layout line at the top of the text area, counted from 1.",
        run: get_view_top,
    },
    EditorFunc {
        name: "ChangeTabWidth",
        params: &[req("width", Ty::Int)],
        result: Ty::Int,
        doc: "The tab width before the call. A width from 1 to 64 that differs from it becomes \
              the tab width of this window (not saved). Any other value, such as 0, only reads.",
        run: change_tab_width,
    },
    EditorFunc {
        name: "ChangeWrapColm",
        params: &[req("column", Ty::Int)],
        result: Ty::Int,
        doc: "The wrap column before the call. A column from 10 to 10240 makes this window wrap \
              at that column (not saved). Any other value only reads. Same as ChangeWrapColumn.",
        run: change_wrap_column,
    },
    EditorFunc {
        name: "ChangeWrapColumn",
        params: &[req("column", Ty::Int)],
        result: Ty::Int,
        doc: "The wrap column before the call. A column from 10 to 10240 makes this window wrap \
              at that column (not saved). Any other value only reads.",
        run: change_wrap_column,
    },
];

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::testing::Fixture;

    #[test]
    fn layout_and_logic_lines_are_the_same_without_wrapping() {
        // "a\n" has one line; the empty line after the break is the layout line 2.
        let mut f = Fixture::new("a\nbb\n日本\n");
        for n in 1..=3 {
            assert_eq!(f.ok("LayoutToLogicLineNum", json!([n])), json!(n));
            assert_eq!(f.ok("LogicToLayoutLineNum", json!([n, 1])), json!(n));
        }
        // The line after the last: the line after the last logical line.
        assert_eq!(f.ok("LayoutToLogicLineNum", json!([4])), json!(4));
        assert_eq!(f.ok("LogicToLayoutLineNum", json!([4, 1])), json!(4));
        assert_eq!(f.ok("LogicToLayoutLineNum", json!([9, 1])), json!(4));
        // Outside: an error, as Sakura's `return false`.
        assert!(f.call("LayoutToLogicLineNum", json!([5])).is_err());
        assert!(f.call("LayoutToLogicLineNum", json!([0])).is_err());
        assert!(f.call("LogicToLayoutLineNum", json!([0, 1])).is_err());
    }

    #[test]
    fn extreme_arguments_are_errors_or_clamped_not_panics() {
        let mut f = Fixture::new("abc\ndef");
        for name in [
            "LineColumnToIndex",
            "LogicToLayoutLineNum",
            "LineIndexToColumn",
        ] {
            assert!(f.call(name, json!([i64::MIN, 1])).is_err(), "{name}");
            assert!(f.call(name, json!([1, i64::MIN])).is_ok(), "{name}");
            assert!(f.call(name, json!([i64::MAX, i64::MAX])).is_ok(), "{name}");
        }
        assert!(f.call("LayoutToLogicLineNum", json!([i64::MIN])).is_err());
        assert!(f.call("LayoutToLogicLineNum", json!([i64::MAX])).is_err());
        assert!(f.call("GetStrWidth", json!(["a\tb", i64::MAX])).is_ok());
        assert!(f.call("GetStrWidth", json!(["a\tb", i64::MIN])).is_ok());
        assert!(
            f.call("GetStrLayoutLength", json!(["a\n", i64::MIN]))
                .is_ok()
        );
    }

    #[test]
    fn only_the_layout_functions_need_exact_rows() {
        use super::super::needs_exact_layout;
        assert!(needs_exact_layout("LogicToLayoutLineNum"));
        assert!(needs_exact_layout("GetSelectColmTo"));
        assert!(!needs_exact_layout("GetFilename"));
        assert!(!needs_exact_layout("GetCookie"));
        assert!(!needs_exact_layout("GetStrWidth"));
    }

    #[test]
    fn an_empty_document_has_no_lines() {
        let mut f = Fixture::new("");
        assert_eq!(f.ok("LayoutToLogicLineNum", json!([1])), json!(1));
        assert!(f.call("LayoutToLogicLineNum", json!([2])).is_err());
    }

    #[test]
    fn indexes_count_utf16_and_columns_count_cells() {
        // "日a\tb": 日 is 2 cells and 1 unit; the tab goes from column 3 to 4.
        let mut f = Fixture::new("日a\tb");
        let col = |f: &mut Fixture, idx: i64| f.ok("LineIndexToColumn", json!([1, idx]));
        assert_eq!(col(&mut f, 1), json!(1));
        assert_eq!(col(&mut f, 2), json!(3));
        assert_eq!(col(&mut f, 3), json!(4));
        assert_eq!(col(&mut f, 4), json!(5));
        assert_eq!(col(&mut f, 5), json!(6));
        let idx = |f: &mut Fixture, col: i64| f.ok("LineColumnToIndex", json!([1, col]));
        assert_eq!(idx(&mut f, 1), json!(1));
        // the second cell of 日 is still 日
        assert_eq!(idx(&mut f, 2), json!(1));
        assert_eq!(idx(&mut f, 3), json!(2));
        // inside the tab
        assert_eq!(idx(&mut f, 4), json!(3));
        assert_eq!(idx(&mut f, 5), json!(4));
        // past the end: the end of the text
        assert_eq!(idx(&mut f, 99), json!(5));
    }

    #[test]
    fn a_character_outside_the_bmp_is_two_units_and_two_cells() {
        let mut f = Fixture::new("a😀b");
        assert_eq!(f.ok("LineIndexToColumn", json!([1, 2])), json!(2));
        assert_eq!(f.ok("LineIndexToColumn", json!([1, 4])), json!(4));
        assert_eq!(f.ok("LineColumnToIndex", json!([1, 4])), json!(4));
    }

    #[test]
    fn an_index_after_the_line_gives_the_line_ending_cell() {
        let mut f = Fixture::new("ab\r\ncd");
        // at the end of the text
        assert_eq!(f.ok("LineIndexToColumn", json!([1, 3])), json!(3));
        // after it, on a line with a line ending: one more cell for its mark
        assert_eq!(f.ok("LineIndexToColumn", json!([1, 9])), json!(4));
        // the last line has no line ending
        assert_eq!(f.ok("LineIndexToColumn", json!([2, 9])), json!(3));
    }

    #[test]
    fn crlf_lines_and_the_line_after_the_last() {
        let mut f = Fixture::new("a\r\nb\r\n");
        assert_eq!(f.ok("LayoutToLogicLineNum", json!([2])), json!(2));
        assert_eq!(f.ok("LayoutToLogicLineNum", json!([3])), json!(3));
        assert_eq!(f.ok("LineColumnToIndex", json!([3, 1])), json!(1));
        assert_eq!(f.ok("LineColumnToIndex", json!([9, 5])), json!(1));
    }

    #[test]
    fn the_width_of_a_string_follows_tabs_and_full_width_characters() {
        let mut f = Fixture::new("");
        assert_eq!(f.ok("GetStrWidth", json!(["abc"])), json!(3));
        assert_eq!(f.ok("GetStrWidth", json!(["日本語"])), json!(6));
        // a tab from column 1 goes to the stop at 4 (width 4): 4 cells
        assert_eq!(f.ok("GetStrWidth", json!(["\t"])), json!(4));
        // from column 3 it goes to the stop at 4: 2 cells
        assert_eq!(f.ok("GetStrWidth", json!(["\t", 3])), json!(2));
        assert_eq!(f.ok("GetStrWidth", json!(["a\tb", 1])), json!(5));
        assert_eq!(f.ok("GetStrWidth", json!([""])), json!(0));
        // CR LF at the end counts as the CR: one cell; LF alone is a character as well
        assert_eq!(f.ok("GetStrWidth", json!(["ab\r\n"])), json!(3));
        assert_eq!(f.ok("GetStrWidth", json!(["ab\n"])), json!(3));
        f.layout.tab_width = 8;
        assert_eq!(f.ok("GetStrWidth", json!(["\t"])), json!(8));
    }

    #[test]
    fn the_layout_length_counts_a_final_line_ending_as_one_cell() {
        let mut f = Fixture::new("");
        assert_eq!(f.ok("GetStrLayoutLength", json!(["abc"])), json!(3));
        assert_eq!(f.ok("GetStrLayoutLength", json!(["abc\r\n"])), json!(4));
        assert_eq!(f.ok("GetStrLayoutLength", json!(["abc\n"])), json!(4));
        // from column 2: 日 reaches column 4 (cells 2 and 3), the tab ends there too (stop at 4)
        assert_eq!(f.ok("GetStrLayoutLength", json!(["日\t", 2])), json!(3));
        assert_eq!(f.ok("GetStrLayoutLength", json!(["日\t\n", 2])), json!(4));
    }

    #[test]
    fn the_view_and_the_default_character() {
        let mut f = Fixture::new("");
        assert_eq!(f.ok("GetDefaultCharLength", json!([])), json!(1));
        assert_eq!(f.ok("GetViewLines", json!([])), json!(30));
        assert_eq!(f.ok("GetViewColumns", json!([])), json!(80));
        assert_eq!(f.ok("GetViewTop", json!([])), json!(1));
        f.env.view = None;
        for name in ["GetViewLines", "GetViewColumns", "GetViewTop"] {
            assert_eq!(
                f.call(name, json!([])).unwrap_err().code,
                crate::bridge::UNAVAILABLE
            );
        }
    }

    #[test]
    fn the_tab_width_is_read_and_changed_by_request() {
        let mut f = Fixture::new("");
        // 0 and out of range only read
        for width in [0, -1, 65] {
            assert_eq!(f.ok("ChangeTabWidth", json!([width])), json!(4));
            assert_eq!(f.env.requests.tab_width, None, "{width}");
        }
        // the width in use is asked for as well (see the function)
        assert_eq!(f.ok("ChangeTabWidth", json!([4])), json!(4));
        assert_eq!(f.env.requests.tab_width, Some(4));
        f.env.requests.tab_width = None;
        assert_eq!(f.ok("ChangeTabWidth", json!([8])), json!(4));
        assert_eq!(f.env.requests.tab_width, Some(8));
    }

    #[test]
    fn the_insert_space_setting_is_read_and_changed_by_request() {
        let mut f = Fixture::new("");
        assert_eq!(
            f.call("ChangeInsertSpace", json!([1])).unwrap_err().code,
            crate::bridge::UNAVAILABLE
        );
        f.env.ins_space = Some(false);
        // other values only read
        for value in [2, -1, 100] {
            assert_eq!(f.ok("ChangeInsertSpace", json!([value])), json!(0));
            assert_eq!(f.env.requests.ins_space, None, "{value}");
        }
        assert_eq!(f.ok("ChangeInsertSpace", json!([1])), json!(0));
        assert_eq!(f.env.requests.ins_space, Some(true));
        f.env.ins_space = Some(true);
        assert_eq!(f.ok("ChangeInsertSpace", json!([0])), json!(1));
        assert_eq!(f.env.requests.ins_space, Some(false));
        // Sakura looks at the low 16 bits (VARIANT::iVal)
        f.env.requests.ins_space = None;
        assert_eq!(f.ok("ChangeInsertSpace", json!([65536])), json!(1));
        assert_eq!(f.env.requests.ins_space, Some(false));
        assert_eq!(f.ok("ChangeInsertSpace", json!([65537])), json!(1));
        assert_eq!(f.env.requests.ins_space, Some(true));
    }

    #[test]
    fn the_wrap_column_is_read_and_changed_by_request() {
        let mut f = Fixture::new("");
        assert_eq!(
            f.call("ChangeWrapColm", json!([0])).unwrap_err().code,
            crate::bridge::UNAVAILABLE
        );
        f.env.wrap_columns = Some(80);
        for column in [0, 9, 10241, -5] {
            assert_eq!(f.ok("ChangeWrapColumn", json!([column])), json!(80));
            assert_eq!(f.env.requests.wrap_columns, None, "{column}");
        }
        assert_eq!(f.ok("ChangeWrapColm", json!([10])), json!(80));
        assert_eq!(f.env.requests.wrap_columns, Some(10));
        assert_eq!(f.ok("ChangeWrapColumn", json!([10240])), json!(80));
        assert_eq!(f.env.requests.wrap_columns, Some(10240));
    }
}
