//! The jumps to the next and the previous modified line (`F_MODIFYLINE_NEXT`,
//! `F_MODIFYLINE_PREV` and their selecting forms). Which lines are modified is kept in the
//! document ([`ume_core::modified`]).
//!
//! Sakura: `Command_MODIFYLINE_NEXT` and `Command_MODIFYLINE_PREV` in
//! `sakura_core/cmd/CViewCommander_Cursor.cpp`. They stop at the first line of a block of modified
//! lines and at the line after its last one (the end of the block), so a block has two stops.

use ume_core::buffer::Pos;
use ume_core::search::Direction;

use super::CommandContext;
use crate::command::Result;

/// How a jump to a modified line ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifiedLineJump {
    /// The caret is on the stop now. `wrapped`: the search went on from the other end of the
    /// file (Sakura says so in the status bar: `STR_ERR_SRNEXT1`, `STR_ERR_SRPREV1`).
    Found { wrapped: bool },
    /// There is no stop in that direction.
    NotFound,
    /// The document has no line, and Sakura says nothing then.
    Empty,
}

impl CommandContext<'_> {
    /// The length of the text of `line` without its line ending (`GetLengthWithoutEOL`), and
    /// whether it has a line ending.
    fn line_shape(&self, line: usize) -> Result<Option<(usize, bool)>> {
        Ok(self.doc().line(line)?.map(|l| (l.text.len(), l.eol.is_some())))
    }

    /// `F_MODIFYLINE_NEXT` / `F_MODIFYLINE_PREV` (and the `_SEL` forms with `extend`): moves the
    /// caret to the next (previous) edge of a block of modified lines, as a plain move (it extends
    /// the selection while the selection is locked, and it is not put in the jump history). With
    /// `wrap` (Sakura's option 先頭（末尾）から再検索, `m_bSearchAll`) a search that finds nothing goes
    /// on from the other end of the file.
    pub fn jump_to_modified_line(
        &mut self,
        direction: Direction,
        extend: bool,
        wrap: bool,
    ) -> Result<ModifiedLineJump> {
        let extend = self.extends(extend);
        let stop = match direction {
            Direction::Forward => self.next_modified_stop(wrap)?,
            Direction::Backward => self.prev_modified_stop(wrap)?,
        };
        match stop {
            Stop::Found(pos, wrapped) => {
                self.move_caret(pos, extend)?;
                Ok(ModifiedLineJump::Found { wrapped })
            }
            Stop::NotFound => Ok(ModifiedLineJump::NotFound),
            Stop::Empty => Ok(ModifiedLineJump::Empty),
        }
    }

    /// `Command_MODIFYLINE_NEXT`. Compares each line with the state of the line the caret is on
    /// (not modified, when the caret is below the last line), and stops where it changes.
    fn next_modified_stop(&self, wrap: bool) -> Result<Stop> {
        let count = self.doc().line_count();
        if count == 0 {
            return Ok(Stop::Empty);
        }
        let modified = |line: usize| self.model.is_line_modified(line);
        let y_old = self.caret().line;
        let mut y = y_old;
        let mut state = false;
        if y < count {
            state = modified(y);
            y += 1;
        }
        for pass in 0..2 {
            while y < count {
                let now = modified(y);
                // `0 == ptXY.y && modified`: a modified first line is a stop of its own, which
                // only the second pass can reach
                if now != state || (y == 0 && now) {
                    return Ok(Stop::Found(Pos::new(y, 0), y_old >= y));
                }
                y += 1;
            }
            if pass == 0 && state {
                // modified down to the end: the end of the text is the end of the block
                let last = count - 1;
                if let Some((len, has_eol)) = self.line_shape(last)? {
                    let (pos, skip) = if has_eol {
                        // the empty row after the last line break (`ptXY`)
                        (Pos::new(y, 0), false)
                    } else {
                        // the end of the last line, which has no line break
                        let pos = Pos::new(last, len);
                        (pos, self.caret() == pos)
                    };
                    if !skip {
                        return Ok(Stop::Found(pos, false));
                    }
                }
            }
            if pass == 0 {
                y = 0;
                state = false;
            }
            if !wrap {
                break;
            }
        }
        Ok(Stop::NotFound)
    }

    /// `Command_MODIFYLINE_PREV`: the mirror image. It stops at the line after the one where the
    /// state changes, so that it lands on the same places as the move forward.
    fn prev_modified_stop(&self, wrap: bool) -> Result<Stop> {
        let count = self.doc().line_count();
        let modified = |line: isize| {
            usize::try_from(line).is_ok_and(|l| l < count && self.model.is_line_modified(l))
        };
        let caret = self.caret();
        let mut y = caret.line as isize;
        // `bLast`: the caret is at the end of the text, so the line it is on is the last
        // line and counts as the first one to compare
        let mut last = false;
        if caret.line >= count {
            // below the last line
            if count == 0 {
                return Ok(Stop::Empty);
            }
            y = count as isize - 1;
            last = true;
        } else if let Some((len, false)) = self.line_shape(count - 1)?
            && caret == Pos::new(count - 1, len)
        {
            // the end of a last line that has no line break counts as the end of the text
            last = true;
        }
        let mut state = modified(y);
        let mut y2 = y;
        y -= 1;
        if y >= 0 && !last {
            state = modified(y);
            y2 = y;
            y -= 1;
        }
        for pass in 0..2 {
            while y >= 0 {
                if modified(y) != state {
                    return Ok(Stop::Found(Pos::new(y2 as usize, 0), pass == 1));
                }
                y2 = y;
                y -= 1;
            }
            if pass == 0 {
                // the first line
                if modified(0) && caret != Pos::new(0, 0) {
                    return Ok(Stop::Found(Pos::new(0, 0), false));
                }
                y = count as isize - 1;
                y2 = y;
                state = false;
            }
            if !wrap {
                break;
            }
            if pass == 0 && modified(count as isize - 1) {
                // a modified last line: the end of the text is a stop
                let last = count - 1;
                if let Some((len, has_eol)) = self.line_shape(last)? {
                    // Sakura puts the position of a last line with a line break one row below the
                    // empty row after it; the move lands on that row all the same
                    let pos = if has_eol {
                        Pos::new(count, 0)
                    } else {
                        Pos::new(last, len)
                    };
                    if caret != pos {
                        return Ok(Stop::Found(pos, true));
                    }
                }
            }
        }
        Ok(Stop::NotFound)
    }
}

/// Where a search for a modified line ended.
enum Stop {
    /// The position, and whether the search wrapped around.
    Found(Pos, bool),
    NotFound,
    Empty,
}
