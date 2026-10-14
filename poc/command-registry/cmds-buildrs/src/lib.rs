//! Option (c): `build.rs` scans `src/commands/<group>/<name>.rs` and writes the module tree and
//! the list. A command file contains no registration line and no `mod` line.

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

// Generated: `pub mod commands { ... }` and `pub static ALL: &[CommandEntry]`.
include!(concat!(env!("OUT_DIR"), "/commands_generated.rs"));

/// Every command, in file-name order.
pub fn all() -> impl Iterator<Item = &'static CommandEntry> {
    ALL.iter()
}
