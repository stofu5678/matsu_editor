//! The terminal behind MATSU Editor's shell tab (ADR-0028), with no UI.
//!
//! It wraps `alacritty_terminal`: the shell runs in a PTY (ConPTY on Windows), a reader thread feeds the
//! terminal state, and the UI copies what it needs out of that state ([`Terminal::info`],
//! [`Terminal::read_lines`], [`Terminal::snapshot`]) while holding the lock only for the copy.
//!
//! What the UI gets:
//!
//! - [`Terminal::spawn`] returns the terminal and an [`EventReceiver`] with the [`TerminalEvent`]s: redraw,
//!   the cleaned title, bell, clipboard store (OSC 52 writes) and child exit. OSC 52 reads are refused.
//! - [`Terminal::start_selection`] and the rest select cells and give the selected text; the selection is in
//!   [`Snapshot::selection`] for drawing.
//! - [`encode_key`], [`encode_paste`] and [`encode_mouse`] turn keys, pasted text and mouse events into the bytes
//!   to [`Terminal::write`]; [`link_at`] finds the link under a cell.
//! - Before the editor ends, [`wait_for_closing`] waits (up to a limit) for the terminals that were closed,
//!   so that their shells end with the editor.
//!
//! MATSU's own environment is never changed (no `std::env::set_var`): the child gets `TERM` and
//! `COLORTERM` through the spawn options only. See [`shell`] for what `alacritty_terminal` cannot leave out.

mod keys;
mod link;
mod mouse;
mod paste;
mod screen;
mod selection;
pub mod shell;
mod terminal;
mod title;

pub use keys::{Key, Modifiers, encode_key};
pub use link::{Link, LinkSpan, display_uri, is_openable, link_at};
pub use mouse::{MouseAction, MouseButton, WheelDirection, encode_mouse, encode_wheel_as_arrows};
pub use paste::{PasteEncoding, encode_paste};
pub use screen::{Cell, ClipboardKind, Color, Cursor, CursorShape, Modes, Rgb, Row, ScreenInfo};
pub use selection::{CellSide, SelectionMode, SelectionSpan, WORD_DELIMITERS};
pub use shell::{CommandLineError, ShellCommand};
pub use terminal::{
    DEFAULT_SCROLLBACK_LINES, EventReceiver, Options, Snapshot, Terminal, TerminalEvent,
    TerminalSize, wait_for_closing,
};
pub use title::{MAX_TITLE_CHARS, clean_title};
