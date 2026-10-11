//! Option (b): `linkme`. Each command file adds an element to one distributed slice.

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

#[linkme::distributed_slice]
pub static COMMANDS: [CommandEntry];

/// Registers a command. Written at the end of the command's own file.
#[macro_export]
macro_rules! register_command {
    ($ty:ty) => {
        const _: () = {
            #[$crate::__linkme::distributed_slice($crate::COMMANDS)]
            #[linkme(crate = $crate::__linkme)]
            static ENTRY: $crate::CommandEntry = $crate::CommandEntry::of::<$ty>();
        };
    };
}

#[doc(hidden)]
pub use linkme as __linkme;

/// Every registered command, in no particular order.
pub fn all() -> impl Iterator<Item = &'static CommandEntry> {
    COMMANDS.iter()
}
