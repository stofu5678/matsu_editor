//! The [`Command`] trait and how a command registers itself.
//!
//! A command is a unit struct in its own file under `src/commands/<group>/`:
//!
//! ```
//! use ume_commands::command::{Arg, Command, Result};
//! use ume_commands::context::CommandContext;
//! use ume_commands::register_command;
//!
//! #[derive(Default)]
//! pub struct GoFileTop;
//!
//! impl Command for GoFileTop {
//!     const ID: &'static str = "go_file_top";
//!     const SAKURA: &'static str = "F_GOFILETOP";
//!
//!     fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
//!         cx.move_file_top(false)
//!     }
//! }
//!
//! register_command!(GoFileTop);
//! ```
//!
//! `register_command!` is the only registration (ADR-0013 section 1, `inventory`). There is no
//! list of commands to edit. The key map, menus and macros call a command through the
//! [registry](crate::registry) by its ID or by Sakura's function code.

use std::fmt;

use ume_core::buffer::BufferError;

use crate::context::CommandContext;

/// An argument of a command. Sakura passes up to four values (`lparam1`..`lparam4`); macros and
/// the JSON-RPC calls will pass the same kinds.
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Int(i64),
    Char(char),
    Text(String),
}

/// Why a command failed. A command that cannot do what was asked in the current state (for
/// example Sakura's `ErrorBeep()` cases) does not fail; it calls
/// [`CommandContext::beep`] and returns `Ok`.
#[derive(Debug)]
pub enum CommandError {
    /// Reading or editing the document failed.
    Buffer(BufferError),
    /// The arguments do not fit the command (a script's mistake).
    BadArgs(String),
    /// No command has this ID or function code.
    Unknown(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buffer(e) => write!(f, "{e}"),
            Self::BadArgs(m) => write!(f, "bad arguments: {m}"),
            Self::Unknown(what) => write!(f, "no such command: {what}"),
        }
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Buffer(e) => Some(e),
            _ => None,
        }
    }
}

impl From<BufferError> for CommandError {
    fn from(e: BufferError) -> Self {
        Self::Buffer(e)
    }
}

pub type Result<T = ()> = std::result::Result<T, CommandError>;

/// A named command.
pub trait Command: Default + 'static {
    /// The command's ID: lowercase `snake_case`, readable (`file_save`, not Sakura's `filesave`).
    /// Unique among all commands. Key maps, menus and macros refer to it.
    const ID: &'static str;

    /// Sakura's name of the function that this command implements, for example `"F_DELETE_BACK"`.
    /// It must be a name in [`FUNCTIONS`](crate::sakura::FUNCTIONS); the registry test checks it
    /// and finds the function code from it. One Sakura function has one command.
    const SAKURA: &'static str;

    /// Runs the command. Edits and moves go through `cx`, which records undo and the changes.
    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result;
}

type RunFn = fn(&mut CommandContext<'_>, &[Arg]) -> Result;

fn run_default<C: Command>(cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
    C::default().run(cx, args)
}

/// What `register_command!` submits for a command.
pub struct Registration {
    pub id: &'static str,
    pub sakura: &'static str,
    /// The file that registered the command, for messages about duplicates.
    pub source: &'static str,
    func: RunFn,
}

impl Registration {
    #[doc(hidden)]
    pub const fn of<C: Command>(source: &'static str) -> Self {
        Self {
            id: C::ID,
            sakura: C::SAKURA,
            source,
            func: run_default::<C>,
        }
    }

    /// Runs the command as one undo block: the edits it makes are one undo step.
    pub fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        cx.begin_block();
        let result = (self.func)(cx, args);
        cx.end_block();
        result
    }
}

impl fmt::Debug for Registration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registration")
            .field("id", &self.id)
            .field("sakura", &self.sakura)
            .finish()
    }
}

inventory::collect!(Registration);

/// Registers a command (a type that implements [`Command`]) with the registry. Put it right
/// after the `impl Command` in the command's file.
#[macro_export]
macro_rules! register_command {
    ($command:ty) => {
        $crate::inventory::submit! {
            $crate::command::Registration::of::<$command>(file!())
        }
    };
}
