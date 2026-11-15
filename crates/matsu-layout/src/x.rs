//! The unit of the horizontal position in a layout (ADR-0034).

use std::iter::Sum;
use std::ops::{Add, AddAssign, Rem, Sub};

/// A horizontal position or width in the layout, counted in a unit of its own and not in columns.
///
/// One column (a half-width character plus the character spacing) is
/// [`Widths::per_column`](crate::Widths::per_column) of these. With the 1 that the grid has today
/// an `X` and a column count are the same number, which is why the type is separate: a later
/// stage gives the unit a finer size (ADR-0034, 1/64 of a half-width column), and a column count
/// used as an `X` would then be wrong without the compiler noticing.
///
/// The only conversions between columns and `X` are [`Widths::to_x`](crate::Widths::to_x) and
/// [`Widths::to_columns`](crate::Widths::to_columns). [`X::from_raw`] and [`X::raw`] are for
/// code that stores or measures the unit itself (a font table, a test).
///
/// Addition saturates, so that a wrap width of `u32::MAX` columns (a row that never wraps) cannot
/// overflow. Subtraction is the plain `u32` one: use [`X::saturating_sub`] when the left side can
/// be smaller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct X(u32);

impl X {
    pub const ZERO: Self = Self(0);

    /// The number itself, in the unit of `X`. Not a column count (see the type's documentation).
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// The number in the unit of `X`.
    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// `self` times a count of characters (a width times a number of equal characters).
    pub const fn saturating_mul(self, times: u32) -> Self {
        Self(self.0.saturating_mul(times))
    }
}

impl Add for X {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl AddAssign for X {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Sub for X {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

/// The remainder of one position by a width, for the tab stops (`tab - x % tab`).
impl Rem for X {
    type Output = Self;

    fn rem(self, other: Self) -> Self {
        Self(self.0 % other.0)
    }
}

impl Sum for X {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, Add::add)
    }
}
