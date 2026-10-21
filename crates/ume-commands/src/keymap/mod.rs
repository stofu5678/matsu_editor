//! Keys, modifiers and the key map.
//!
//! * `key`: [`Key`], [`Modifiers`] and [`Chord`], with Sakura's key names for menus and
//!   settings.
//! * [`KeyMap`]: Sakura's default key table (from `crate::sakura::KEY_BINDINGS`) as a lookup
//!   from a chord to a command, with the macOS rules of ADR-0008 ([`KeyMapOptions`]).
//! * [`user`]: the user's key bindings as Sakura's `KeyBind` entries.
//!
//! The UI converts its key events to [`Chord`]s and asks [`KeyMap::lookup`]; this crate has no
//! UI types.

mod bind_list;
mod key;
mod map;
mod options;
pub mod user;

pub use bind_list::create_key_bind_list;
pub use key::{Chord, DisplayStyle, Key, Modifiers, MouseAction};
pub use map::{KeyMap, Lookup, Resolved};
pub use options::{ControlKey, KeyMapOptions, Platform};
pub use user::UserRow;

#[cfg(test)]
mod tests;
