//! Option (a): the `inventory` crate. Each command file submits an entry before `main` starts.

pub mod commands;

/// What a command receives. The real one is still open (ADR-0013).
#[derive(Default)]
pub struct CommandContext {
    pub log: Vec<String>,
}

pub type Result<T> = std::result::Result<T, String>;

/// A command: one struct per file, called by its `ID`.
pub trait Command {
    const ID: &'static str;
    fn run(&self, cx: &mut CommandContext) -> Result<()>;
}

/// What the registry stores for one command.
pub struct CommandEntry {
    pub id: &'static str,
    pub run: fn(&mut CommandContext) -> Result<()>,
}

impl CommandEntry {
    pub const fn of<C: Command + Default>() -> Self {
        fn run_default<C: Command + Default>(cx: &mut CommandContext) -> Result<()> {
            C::default().run(cx)
        }
        Self {
            id: C::ID,
            run: run_default::<C>,
        }
    }
}

inventory::collect!(CommandEntry);

/// Registers a command. Written at the end of the command's own file.
#[macro_export]
macro_rules! register_command {
    ($ty:ty) => {
        $crate::__inventory::submit! { $crate::CommandEntry::of::<$ty>() }
    };
}

#[doc(hidden)]
pub use inventory as __inventory;

/// Every registered command, in no particular order.
pub fn all() -> impl Iterator<Item = &'static CommandEntry> {
    inventory::iter::<CommandEntry>.into_iter()
}
