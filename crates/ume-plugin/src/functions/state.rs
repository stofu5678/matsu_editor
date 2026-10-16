//! The state of the document and of the selection: the layout position of the selection, the
//! typing mode, undo, the line ending, the encoding and the bookmarks.

use serde_json::Value;
use ume_core::encoding::Eol;

use super::{EditorFunc, layout::layout_point, unavailable};
use crate::api::{Args, Ty, req};
use crate::bridge::NOT_IMPLEMENTED;
use crate::host::Editor;
use crate::protocol::RpcError;

type Result<T = Value> = std::result::Result<T, RpcError>;

/// The corners of the selection in layout units, `(line, column)` counted from 0 (the `m_sSelect`
/// of `CViewSelect`, a `CLayoutRange`): the first and then the last, the smaller one first.
///
/// For a box selection they are the two corners of the rectangle as the selection keeps them,
/// ordered like points (by line, then by column), not as a rectangle (`CViewSelect::
/// ChangeSelectAreaByCurrentCursorTEST`). `None` when nothing is selected, where Sakura's range
/// is cleared to -1 (`CViewSelect::DisableSelectArea`) and the functions answer 0.
/// A point in layout units: `(line, column)`, from 0.
type Point = (usize, usize);

fn selection_corners(ed: &Editor) -> Result<Option<(Point, Point)>> {
    let cx = &*ed.cx;
    let a;
    let b;
    if let Some(((pa, ca), (pb, cb))) = cx.cursor().box_corners() {
        a = (layout_point(cx, pa)?.0, ca);
        b = (layout_point(cx, pb)?.0, cb);
    } else if let Some((from, to)) = cx.selection() {
        a = layout_point(cx, from)?;
        b = layout_point(cx, to)?;
    } else {
        return Ok(None);
    }
    Ok(Some(if a <= b { (a, b) } else { (b, a) }))
}

/// `1 + the value`, or 0 when nothing is selected (Sakura: `-1 + 1`).
fn corner(ed: &Editor, last: bool, column: bool) -> Result {
    let Some((first, second)) = selection_corners(ed)? else {
        return Ok(0.into());
    };
    let (line, col) = if last { second } else { first };
    Ok((if column { col } else { line } as i64 + 1).into())
}

/// F_GETSELLINEFROM: the line of the start of the selection, counted from 1 in layout lines.
fn sel_line_from(ed: &mut Editor, _: &Args) -> Result {
    corner(ed, false, false)
}

/// F_GETSELCOLUMNFROM: the column of the start of the selection, counted from 1 in layout
/// columns (cells; the indent of a wrapped row is included).
fn sel_column_from(ed: &mut Editor, _: &Args) -> Result {
    corner(ed, false, true)
}

/// F_GETSELLINETO.
fn sel_line_to(ed: &mut Editor, _: &Args) -> Result {
    corner(ed, true, false)
}

/// F_GETSELCOLUMNTO.
fn sel_column_to(ed: &mut Editor, _: &Args) -> Result {
    corner(ed, true, true)
}

/// F_ISINSMODE: 1 for insert mode, 0 for overwrite mode (`CEditView::IsInsMode`).
fn is_ins_mode(ed: &mut Editor, _: &Args) -> Result {
    Ok(i64::from(!ed.cx.is_overwrite()).into())
}

/// F_ISPOSSIBLEUNDO (`CDocEditor::IsEnableUndo`).
fn is_possible_undo(ed: &mut Editor, _: &Args) -> Result {
    Ok(i64::from(ed.cx.can_undo()).into())
}

/// F_ISPOSSIBLEREDO (`CDocEditor::IsEnableRedo`).
fn is_possible_redo(ed: &mut Editor, _: &Args) -> Result {
    Ok(i64::from(ed.cx.can_redo()).into())
}

/// F_GETLINECODE: the line ending that Enter inserts (`CDocEditor::GetNewLineCode`): 0 CR LF,
/// 1 CR, 2 LF, and the extended endings 3 NEL, 4 LS, 5 PS.
fn get_line_code(ed: &mut Editor, _: &Args) -> Result {
    Ok(match ed.cx.new_line() {
        Eol::CrLf => 0,
        Eol::Cr => 1,
        Eol::Lf => 2,
        Eol::Nel => 3,
        Eol::Ls => 4,
        Eol::Ps => 5,
    }
    .into())
}

/// F_GETCHARCODE: Sakura's code of the encoding of the file (`GetDocumentEncoding`): 0 Shift_JIS,
/// 1 JIS, 2 EUC-JP, 3 UTF-16, 4 UTF-8, 5 UTF-7, 6 UTF-16 BE, 7 CESU-8, 8 Latin1. The code pages
/// that Sakura has no number in this list for (such as GBK) answer their own code of the settings
/// file (`Encoding::sakura_code`).
fn get_char_code(ed: &mut Editor, _: &Args) -> Result {
    ed.env
        .char_code
        .map(Into::into)
        .ok_or_else(|| unavailable("the encoding of the file"))
}

/// F_ISTEXTSELECTINGLOCK: 0 not locked, 1 the selection is locked (the moves extend it), 2
/// locked and a box selection.
fn is_text_selecting_lock(ed: &mut Editor, _: &Args) -> Result {
    let cursor = ed.cx.cursor();
    Ok(if !cursor.selecting_lock() {
        0
    } else if cursor.is_box() {
        2
    } else {
        1
    }
    .into())
}

/// F_GETLINEATTRIBUTE: an attribute of a logical line (0 is the line of the caret; the lines
/// count from 1). The types: 0 modified since the last save, 1 the modification sequence number,
/// 2 bookmark, 3 diff mark, 4 function list mark. Only the bookmark (2) exists in this editor yet;
/// the others answer "not implemented" instead of a made-up 0. A type outside 0 to 4, a negative
/// line, and a line past the end of the text are errors (Sakura: the function fails).
fn get_line_attribute(ed: &mut Editor, args: &Args) -> Result {
    let (number, kind) = (args.int(0), args.int(1));
    if number < 0 {
        return Err(RpcError::invalid_params(
            "GetLineAttribute: the line number must be 0 or more",
        ));
    }
    let line = if number == 0 {
        ed.cx.caret().line
    } else {
        usize::try_from(number - 1).unwrap_or(usize::MAX)
    };
    // The position after the last line break is not a line (`GetLine` is null there).
    let lines = ed.cx.doc().line_count();
    if line >= lines {
        return Err(RpcError::invalid_params(format!(
            "GetLineAttribute: there is no line {number}"
        )));
    }
    match kind {
        2 => Ok(i64::from(ed.cx.is_bookmarked(line)).into()),
        0 | 1 | 3 | 4 => Err(RpcError::new(
            NOT_IMPLEMENTED,
            format!(
                "GetLineAttribute type {kind} ({}) is not supported yet: this editor does not keep it",
                match kind {
                    0 => "modified since the last save",
                    1 => "modification sequence number",
                    3 => "diff mark",
                    _ => "function list mark",
                }
            ),
        )),
        _ => Err(RpcError::invalid_params(
            "GetLineAttribute: the type must be 0 to 4",
        )),
    }
}

/// F_GETSAVEFILENAME: the path that a save writes to. Sakura differs from `GetFilename` only
/// for a file opened through a shortcut (`.lnk`), which does not exist here, so it is the path of
/// the file.
fn get_save_filename(ed: &mut Editor, _: &Args) -> Result {
    Ok(ed
        .cx
        .path()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
        .into())
}

pub const FUNCS: &[EditorFunc] = &[
    EditorFunc {
        name: "GetSelectLineFrom",
        params: &[],
        result: Ty::Int,
        doc: "The line where the selection starts, counted from 1 in layout lines (wrapped \
              rows count as lines). 0 when nothing is selected.",
        run: sel_line_from,
    },
    EditorFunc {
        name: "GetSelectColmFrom",
        params: &[],
        result: Ty::Int,
        doc: "The column where the selection starts, counted from 1 in layout columns (cells: a \
              full-width character is 2, a tab goes to the tab stop). 0 when nothing is \
              selected. Same as GetSelectColumnFrom.",
        run: sel_column_from,
    },
    EditorFunc {
        name: "GetSelectColumnFrom",
        params: &[],
        result: Ty::Int,
        doc: "The column where the selection starts, counted from 1 in layout columns (cells: a \
              full-width character is 2, a tab goes to the tab stop). 0 when nothing is \
              selected.",
        run: sel_column_from,
    },
    EditorFunc {
        name: "GetSelectLineTo",
        params: &[],
        result: Ty::Int,
        doc: "The line where the selection ends, counted from 1 in layout lines. 0 when nothing \
              is selected.",
        run: sel_line_to,
    },
    EditorFunc {
        name: "GetSelectColmTo",
        params: &[],
        result: Ty::Int,
        doc: "The column where the selection ends, counted from 1 in layout columns. 0 when \
              nothing is selected. Same as GetSelectColumnTo.",
        run: sel_column_to,
    },
    EditorFunc {
        name: "GetSelectColumnTo",
        params: &[],
        result: Ty::Int,
        doc: "The column where the selection ends, counted from 1 in layout columns. 0 when \
              nothing is selected.",
        run: sel_column_to,
    },
    EditorFunc {
        name: "IsInsMode",
        params: &[],
        result: Ty::Int,
        doc: "1 in insert mode, 0 in overwrite mode.",
        run: is_ins_mode,
    },
    EditorFunc {
        name: "GetCharCode",
        params: &[],
        result: Ty::Int,
        doc: "The encoding of the file: 0 Shift_JIS, 1 JIS, 2 EUC-JP, 3 UTF-16, 4 UTF-8, \
              5 UTF-7, 6 UTF-16 BE, 7 CESU-8, 8 Latin1.",
        run: get_char_code,
    },
    EditorFunc {
        name: "GetLineCode",
        params: &[],
        result: Ty::Int,
        doc: "The line ending that new line breaks get: 0 CR LF, 1 CR, 2 LF (3 NEL, 4 LS, 5 PS).",
        run: get_line_code,
    },
    EditorFunc {
        name: "IsPossibleUndo",
        params: &[],
        result: Ty::Int,
        doc: "1 when there is something to undo, else 0.",
        run: is_possible_undo,
    },
    EditorFunc {
        name: "IsPossibleRedo",
        params: &[],
        result: Ty::Int,
        doc: "1 when there is something to redo, else 0.",
        run: is_possible_redo,
    },
    EditorFunc {
        name: "GetLineAttribute",
        params: &[req("line", Ty::Int), req("type", Ty::Int)],
        result: Ty::Int,
        doc: "An attribute of a logical line (0 is the line of the cursor, 1 the first line). \
              Type 2 is the bookmark (1 or 0). The other types (0 modified, 1 modification \
              number, 3 diff mark, 4 function list mark) answer the error -32001 until the \
              editor keeps them. A negative line, a line past the end and any other type are \
              errors.",
        run: get_line_attribute,
    },
    EditorFunc {
        name: "IsTextSelectingLock",
        params: &[],
        result: Ty::Int,
        doc: "0 when the selection is not locked, 1 when it is locked (plain moves extend the \
              selection), 2 when it is locked and a box selection.",
        run: is_text_selecting_lock,
    },
    EditorFunc {
        name: "GetSaveFilename",
        params: &[],
        result: Ty::Str,
        doc: "The full path that a save writes to, or an empty string when the text has no \
              file. The same as GetFilename here.",
        run: get_save_filename,
    },
];

#[cfg(test)]
mod tests {
    use serde_json::json;
    use ume_core::buffer::Pos;

    use super::super::testing::Fixture;

    #[test]
    fn nothing_selected_answers_0() {
        let mut f = Fixture::new("abc\ndef");
        for name in [
            "GetSelectLineFrom",
            "GetSelectColmFrom",
            "GetSelectColumnFrom",
            "GetSelectLineTo",
            "GetSelectColmTo",
            "GetSelectColumnTo",
        ] {
            assert_eq!(f.ok(name, json!([])), json!(0), "{name}");
        }
    }

    #[test]
    fn a_selection_is_counted_from_1_in_layout_units() {
        // 日 is 2 cells, the tab goes to column 4 (tab width 4): "日a\tb".
        let mut f = Fixture::new("x\n日a\tbc\nz");
        // from (1, after 日) to (1, after the tab): bytes 3 and 5.
        f.select((1, 3), (1, 5));
        assert_eq!(f.ok("GetSelectLineFrom", json!([])), json!(2));
        assert_eq!(f.ok("GetSelectColmFrom", json!([])), json!(3));
        assert_eq!(f.ok("GetSelectColumnFrom", json!([])), json!(3));
        assert_eq!(f.ok("GetSelectLineTo", json!([])), json!(2));
        assert_eq!(f.ok("GetSelectColmTo", json!([])), json!(5));
        assert_eq!(f.ok("GetSelectColumnTo", json!([])), json!(5));
    }

    #[test]
    fn the_first_corner_is_the_smaller_one_whichever_way_it_was_selected() {
        let mut f = Fixture::new("abc\ndef\nghi");
        // anchor at the end, caret at the start
        f.select((2, 2), (0, 1));
        assert_eq!(f.ok("GetSelectLineFrom", json!([])), json!(1));
        assert_eq!(f.ok("GetSelectColumnFrom", json!([])), json!(2));
        assert_eq!(f.ok("GetSelectLineTo", json!([])), json!(3));
        assert_eq!(f.ok("GetSelectColumnTo", json!([])), json!(3));
    }

    #[test]
    fn a_box_selection_reports_its_corners() {
        let mut f = Fixture::new("abcdef\nabcdef\nabcdef");
        f.cursor.move_to(Pos::new(0, 1), false);
        f.cursor.begin_box(1, false);
        f.cursor.move_box_to(Pos::new(2, 4), 4);
        assert_eq!(f.ok("GetSelectLineFrom", json!([])), json!(1));
        assert_eq!(f.ok("GetSelectColumnFrom", json!([])), json!(2));
        assert_eq!(f.ok("GetSelectLineTo", json!([])), json!(3));
        assert_eq!(f.ok("GetSelectColumnTo", json!([])), json!(5));
    }

    #[test]
    fn the_typing_mode_undo_and_redo() {
        let mut f = Fixture::new("abc");
        assert_eq!(f.ok("IsInsMode", json!([])), json!(1));
        f.overwrite = true;
        assert_eq!(f.ok("IsInsMode", json!([])), json!(0));
        assert_eq!(f.ok("IsPossibleUndo", json!([])), json!(0));
        assert_eq!(f.ok("IsPossibleRedo", json!([])), json!(0));
        // an edit in a block can be undone, and then redone
        f.model.begin_block(&f.cursor);
        f.model
            .replace(Pos::new(0, 0), Pos::new(0, 0), "x")
            .unwrap();
        f.model.end_block(&f.cursor);
        assert_eq!(f.ok("IsPossibleUndo", json!([])), json!(1));
        assert!(f.model.undo(&mut f.cursor));
        assert_eq!(f.ok("IsPossibleUndo", json!([])), json!(0));
        assert_eq!(f.ok("IsPossibleRedo", json!([])), json!(1));
    }

    #[test]
    fn the_line_ending_and_the_encoding() {
        // The line ending of new lines is the first one of the file.
        for (text, code) in [("a\r\nb", 0), ("a\rb", 1), ("a\nb", 2)] {
            let mut f = Fixture::new(text);
            assert_eq!(f.ok("GetLineCode", json!([])), json!(code), "{text:?}");
        }
        let mut f = Fixture::new("a");
        assert_eq!(f.ok("GetCharCode", json!([])), json!(4));
        f.env.char_code = None;
        assert_eq!(
            f.call("GetCharCode", json!([])).unwrap_err().code,
            crate::bridge::UNAVAILABLE
        );
    }

    #[test]
    fn the_selection_lock_has_three_states() {
        let mut f = Fixture::new("abc\ndef");
        assert_eq!(f.ok("IsTextSelectingLock", json!([])), json!(0));
        f.cursor.set_selecting_lock(true);
        assert_eq!(f.ok("IsTextSelectingLock", json!([])), json!(1));
        f.cursor.begin_box(0, true);
        assert_eq!(f.ok("IsTextSelectingLock", json!([])), json!(2));
    }

    #[test]
    fn line_attribute_knows_bookmarks_and_refuses_the_rest() {
        let mut f = Fixture::new("a\nb\nc\n");
        f.model.bookmarks_mut().set(1, true);
        assert_eq!(f.ok("GetLineAttribute", json!([2, 2])), json!(1));
        assert_eq!(f.ok("GetLineAttribute", json!([1, 2])), json!(0));
        // 0 is the line of the caret
        f.caret_at((1, 0));
        assert_eq!(f.ok("GetLineAttribute", json!([0, 2])), json!(1));
        // lines 1 to 3 exist; the empty line after the final line break does not
        assert!(f.call("GetLineAttribute", json!([4, 2])).is_err());
        assert!(f.call("GetLineAttribute", json!([-1, 2])).is_err());
        assert!(f.call("GetLineAttribute", json!([1, 5])).is_err());
        for kind in [0, 1, 3, 4] {
            let e = f.call("GetLineAttribute", json!([1, kind])).unwrap_err();
            assert_eq!(e.code, crate::bridge::NOT_IMPLEMENTED, "{kind}");
        }
    }

    #[test]
    fn the_save_filename_is_the_path_or_empty() {
        let mut f = Fixture::new("a");
        assert_eq!(f.ok("GetSaveFilename", json!([])), json!(""));
        f.path = Some("/tmp/メモ.txt".into());
        assert_eq!(f.ok("GetSaveFilename", json!([])), json!("/tmp/メモ.txt"));
    }
}
