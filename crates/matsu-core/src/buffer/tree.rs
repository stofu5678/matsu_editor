//! A persistent (copy-on-write) AVL tree of pieces.
//!
//! Every node is behind an `Arc`. An edit builds a new path from the root and shares the rest, so
//! cloning a tree is one `Arc` clone and old versions stay valid. Edits are built from `split`
//! and `join` (Blelloch, Ferizovic, Sun: "Just Join for Parallel Ordered Sets"), which keep the
//! tree balanced without in-place rotations.
//!
//! Each node holds one piece and the aggregate of its subtree: the byte length, the number of
//! line breaks, and whether the subtree starts with an LF / ends with a CR. Those last two make
//! a CRLF split over two pieces count as one break (see [`Agg::then`]).

use std::sync::Arc;

use super::codec::Eol;
use super::error::Result;
use super::storage::Storages;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Store {
    Original,
    Add,
}

/// What a run of bytes contains, as far as line breaks are concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Agg {
    pub bytes: usize,
    /// Line breaks counted inside the run, each piece on its own.
    pub breaks: usize,
    /// The length of an LF at the very start of the run, or 0.
    pub first_lf: u8,
    /// Whether the run ends with a lone CR.
    pub last_cr: bool,
}

impl Agg {
    pub const EMPTY: Agg = Agg {
        bytes: 0,
        breaks: 0,
        first_lf: 0,
        last_cr: false,
    };

    /// The aggregate of `self` followed by `next`. A CR at the end of `self` and an LF at the start
    /// of `next` are one CRLF: it was counted twice, so one is taken away.
    pub fn then(self, next: Agg) -> Agg {
        if self.bytes == 0 {
            return next;
        }
        if next.bytes == 0 {
            return self;
        }
        Agg {
            bytes: self.bytes + next.bytes,
            breaks: self.breaks + next.breaks - usize::from(self.last_cr && next.first_lf > 0),
            first_lf: self.first_lf,
            last_cr: next.last_cr,
        }
    }

    /// The breaks of this run when it follows something that does (`prev_cr`) or does not end with
    /// a CR.
    pub fn effective_breaks(&self, prev_cr: bool) -> usize {
        self.breaks - usize::from(prev_cr && self.first_lf > 0)
    }
}

/// A byte range of one storage. Never empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Piece {
    pub store: Store,
    pub start: usize,
    pub end: usize,
    pub agg: Agg,
}

#[derive(Debug)]
pub(crate) struct Node {
    left: Tree,
    right: Tree,
    piece: Piece,
    height: u8,
    agg: Agg,
}

pub(crate) type Tree = Option<Arc<Node>>;

fn height(t: &Tree) -> u8 {
    t.as_ref().map_or(0, |n| n.height)
}

pub(crate) fn agg(t: &Tree) -> Agg {
    t.as_ref().map_or(Agg::EMPTY, |n| n.agg)
}

fn node(left: Tree, piece: Piece, right: Tree) -> Arc<Node> {
    let height = 1 + height(&left).max(height(&right));
    let agg = agg(&left).then(piece.agg).then(agg(&right));
    Arc::new(Node {
        left,
        right,
        piece,
        height,
        agg,
    })
}

fn rotate_left(n: Arc<Node>) -> Arc<Node> {
    match &n.right {
        Some(r) => {
            let inner = node(n.left.clone(), n.piece, r.left.clone());
            node(Some(inner), r.piece, r.right.clone())
        }
        None => n,
    }
}

fn rotate_right(n: Arc<Node>) -> Arc<Node> {
    match &n.left {
        Some(l) => {
            let inner = node(l.right.clone(), n.piece, n.right.clone());
            node(l.left.clone(), l.piece, Some(inner))
        }
        None => n,
    }
}

fn join_right(l: Arc<Node>, p: Piece, r: Tree) -> Arc<Node> {
    let (ll, lp, lr) = (l.left.clone(), l.piece, l.right.clone());
    match lr {
        Some(c) if c.height > height(&r) + 1 => {
            let t1 = join_right(c, p, r);
            let t2 = node(ll.clone(), lp, Some(t1.clone()));
            if t1.height <= height(&ll) + 1 {
                t2
            } else {
                rotate_left(t2)
            }
        }
        lr => {
            let t1 = node(lr, p, r);
            if t1.height <= height(&ll) + 1 {
                node(ll, lp, Some(t1))
            } else {
                rotate_left(node(ll, lp, Some(rotate_right(t1))))
            }
        }
    }
}

fn join_left(l: Tree, p: Piece, r: Arc<Node>) -> Arc<Node> {
    let (rl, rp, rr) = (r.left.clone(), r.piece, r.right.clone());
    match rl {
        Some(c) if c.height > height(&l) + 1 => {
            let t1 = join_left(l, p, c);
            let t2 = node(Some(t1.clone()), rp, rr.clone());
            if t1.height <= height(&rr) + 1 {
                t2
            } else {
                rotate_right(t2)
            }
        }
        rl => {
            let t1 = node(l, p, rl);
            if t1.height <= height(&rr) + 1 {
                node(Some(t1), rp, rr)
            } else {
                rotate_right(node(Some(rotate_left(t1)), rp, rr))
            }
        }
    }
}

/// A balanced tree of `l`, `p`, `r` in this order. Every piece of `l` comes before every piece of `r`.
pub(crate) fn join(l: Tree, p: Piece, r: Tree) -> Arc<Node> {
    match (l, r) {
        (Some(ln), r) if ln.height > height(&r) + 1 => join_right(ln, p, r),
        (l, Some(rn)) if rn.height > height(&l) + 1 => join_left(l, p, rn),
        (l, r) => node(l, p, r),
    }
}

/// Removes the last piece.
fn split_last(t: &Arc<Node>) -> (Tree, Piece) {
    match &t.right {
        None => (t.left.clone(), t.piece),
        Some(r) => {
            let (rest, last) = split_last(r);
            (Some(join(t.left.clone(), t.piece, rest)), last)
        }
    }
}

/// Removes the last piece, if there is one.
pub(crate) fn pop_last(t: &Tree) -> Option<(Tree, Piece)> {
    t.as_ref().map(split_last)
}

pub(crate) fn last_piece(t: &Tree) -> Option<Piece> {
    let mut cur = t.as_ref()?;
    while let Some(r) = &cur.right {
        cur = r;
    }
    Some(cur.piece)
}

pub(crate) fn concat(l: Tree, r: Tree) -> Tree {
    match l {
        None => r,
        Some(ln) => {
            let (rest, last) = split_last(&ln);
            Some(join(rest, last, r))
        }
    }
}

/// Splits the tree at byte offset `at` (counted over all pieces). A piece that contains `at`
/// is cut in two. `at` must be on a character boundary.
pub(crate) fn split(t: &Tree, at: usize, st: &Storages) -> Result<(Tree, Tree)> {
    let Some(n) = t else {
        return Ok((None, None));
    };
    let lb = agg(&n.left).bytes;
    let plen = n.piece.end - n.piece.start;
    Ok(if at <= lb {
        let (a, b) = split(&n.left, at, st)?;
        (a, Some(join(b, n.piece, n.right.clone())))
    } else if at >= lb + plen {
        let (a, b) = split(&n.right, at - lb - plen, st)?;
        (Some(join(n.left.clone(), n.piece, a)), b)
    } else {
        let cut = n.piece.start + (at - lb);
        let p1 = st.piece(n.piece.store, n.piece.start, cut)?;
        let p2 = st.piece(n.piece.store, cut, n.piece.end)?;
        (
            Some(join(n.left.clone(), p1, None)),
            Some(join(None, p2, n.right.clone())),
        )
    })
}

/// The piece that contains the byte at `offset`, and the offset where it starts.
pub(crate) fn piece_at(t: &Tree, offset: usize) -> Option<(Piece, usize)> {
    let mut cur = t.as_ref()?;
    let mut base = 0;
    loop {
        let lb = agg(&cur.left).bytes;
        let plen = cur.piece.end - cur.piece.start;
        if offset < base + lb {
            cur = cur.left.as_ref()?;
        } else if offset < base + lb + plen {
            return Some((cur.piece, base + lb));
        } else {
            base += lb + plen;
            cur = cur.right.as_ref()?;
        }
    }
}

/// A line break as the pieces see it. A CR at the end of a piece may be the first half of a CRLF
/// (`open_cr`); the caller looks at the next piece.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RawBreak {
    pub start: usize,
    pub len: usize,
    pub kind: Eol,
    pub open_cr: bool,
}

/// The `k`-th line break (0-based) of the tree.
pub(crate) fn nth_break(t: &Tree, k: usize, st: &Storages) -> Result<Option<RawBreak>> {
    nth_break_in(t, k, 0, false, st)
}

fn nth_break_in(
    t: &Tree,
    k: usize,
    base: usize,
    prev_cr: bool,
    st: &Storages,
) -> Result<Option<RawBreak>> {
    let Some(n) = t.as_ref() else {
        return Ok(None);
    };
    let la = agg(&n.left);
    let in_left = la.effective_breaks(prev_cr);
    if k < in_left {
        return nth_break_in(&n.left, k, base, prev_cr, st);
    }
    let k = k - in_left;
    let prev_for_piece = if la.bytes > 0 { la.last_cr } else { prev_cr };
    let p = n.piece;
    let skip = usize::from(prev_for_piece && p.agg.first_lf > 0);
    let in_piece = p.agg.breaks - skip;
    let piece_base = base + la.bytes;
    if k < in_piece {
        let Some(v) = st.nth_view(&p, k + skip)? else {
            return Ok(None);
        };
        return Ok(Some(RawBreak {
            start: piece_base + (v.off - p.start),
            len: v.len,
            kind: v.kind,
            open_cr: v.kind == Eol::Cr && v.off + v.len == p.end,
        }));
    }
    nth_break_in(
        &n.right,
        k - in_piece,
        piece_base + p.agg.bytes,
        p.agg.last_cr,
        st,
    )
}

/// Calls `f(piece, start, end, raw_start)` for every piece that overlaps the byte range `a..b`
/// of the document, in order. `start..end` is the overlapping part in the piece's storage and
/// `raw_start` is where it starts in the document.
pub(crate) fn visit<E>(
    t: &Tree,
    a: usize,
    b: usize,
    f: &mut impl FnMut(&Piece, usize, usize, usize) -> std::result::Result<(), E>,
) -> std::result::Result<(), E> {
    visit_in(t, 0, a, b, f)
}

fn visit_in<E>(
    t: &Tree,
    base: usize,
    a: usize,
    b: usize,
    f: &mut impl FnMut(&Piece, usize, usize, usize) -> std::result::Result<(), E>,
) -> std::result::Result<(), E> {
    let Some(n) = t else {
        return Ok(());
    };
    if b <= base || a >= base + n.agg.bytes {
        return Ok(());
    }
    let lb = agg(&n.left).bytes;
    visit_in(&n.left, base, a, b, f)?;
    let ps = base + lb;
    let pe = ps + (n.piece.end - n.piece.start);
    if a < pe && b > ps {
        let from = a.max(ps);
        let to = b.min(pe);
        f(
            &n.piece,
            n.piece.start + (from - ps),
            n.piece.start + (to - ps),
            from,
        )?;
    }
    visit_in(&n.right, pe, a, b, f)
}

/// All pieces in order.
pub(crate) fn pieces(t: &Tree) -> Vec<Piece> {
    let mut out = Vec::new();
    let total = agg(t).bytes;
    let _: Result<()> = visit(t, 0, total, &mut |p, _, _, _| {
        out.push(*p);
        Ok(())
    });
    out
}

#[cfg(test)]
pub(crate) fn check_invariants(t: &Tree, st: &Storages) {
    fn go(t: &Tree, st: &Storages) -> (u8, Agg) {
        let Some(n) = t else {
            return (0, Agg::EMPTY);
        };
        let (hl, al) = go(&n.left, st);
        let (hr, ar) = go(&n.right, st);
        assert!(hl.abs_diff(hr) <= 1, "unbalanced");
        assert_eq!(n.height, 1 + hl.max(hr));
        assert!(n.piece.end > n.piece.start, "empty piece");
        assert_eq!(
            n.piece.agg,
            st.piece(n.piece.store, n.piece.start, n.piece.end)
                .unwrap()
                .agg
        );
        let agg = al.then(n.piece.agg).then(ar);
        assert_eq!(n.agg, agg);
        (n.height, agg)
    }
    go(t, st);
}
