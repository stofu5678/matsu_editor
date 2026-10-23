//! The registry of all commands: lookup by ID and by Sakura function code.
//!
//! The commands register themselves with `register_command!`; this module collects them
//! (`inventory::iter`) and checks them. `inventory` registers nothing, silently, on a target where
//! its linker trick does not work, so the executable must refer to this crate (`use matsu_commands
//! as _;`) and the tests check that the registry is not empty (ADR-0013 section 1, "守ること").

use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use crate::command::{Arg, CommandError, Registration, Result};
use crate::context::CommandContext;
use crate::sakura::{FUNCTIONS, FunctionKind};

/// A mistake in the set of registered commands.
#[derive(Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// An ID is not lowercase `snake_case`.
    BadId {
        id: &'static str,
        source: &'static str,
    },
    /// Two commands have the same ID.
    DuplicateId {
        id: &'static str,
        first: &'static str,
        second: &'static str,
    },
    /// `SAKURA` is not the name of a function in Sakura's table.
    UnknownSakura {
        id: &'static str,
        sakura: &'static str,
        source: &'static str,
    },
    /// `SAKURA` names a function that no menu or key can call (a placeholder or a string ID).
    NotACommand {
        id: &'static str,
        sakura: &'static str,
        source: &'static str,
    },
    /// Two commands implement the same Sakura function.
    DuplicateSakura {
        sakura: &'static str,
        first: &'static str,
        second: &'static str,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadId { id, source } => {
                write!(
                    f,
                    "command ID {id:?} ({source}) is not lowercase snake_case"
                )
            }
            Self::DuplicateId { id, first, second } => {
                write!(
                    f,
                    "command ID {id:?} is registered twice ({first} and {second})"
                )
            }
            Self::UnknownSakura { id, sakura, source } => write!(
                f,
                "command {id:?} ({source}) names {sakura:?}, which is not in Sakura's function table"
            ),
            Self::NotACommand { id, sakura, source } => write!(
                f,
                "command {id:?} ({source}) names {sakura:?}, which is not a function that can be run"
            ),
            Self::DuplicateSakura {
                sakura,
                first,
                second,
            } => write!(
                f,
                "Sakura's {sakura} is implemented twice (command {first:?} and {second:?})"
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

/// All registered commands.
#[derive(Debug)]
pub struct Registry {
    by_id: HashMap<&'static str, &'static Registration>,
    by_code: HashMap<u16, &'static Registration>,
}

fn is_snake_case(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('_')
        && !id.ends_with('_')
        && !id.contains("__")
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !id.starts_with(|c: char| c.is_ascii_digit())
}

impl Registry {
    /// Collects and checks the registered commands.
    pub fn build() -> std::result::Result<Self, RegistryError> {
        Self::from_registrations(inventory::iter::<Registration>)
    }

    fn from_registrations(
        registrations: impl IntoIterator<Item = &'static Registration>,
    ) -> std::result::Result<Self, RegistryError> {
        let mut by_id: HashMap<&'static str, &'static Registration> = HashMap::new();
        let mut by_code: HashMap<u16, &'static Registration> = HashMap::new();
        for reg in registrations {
            if !is_snake_case(reg.id) {
                return Err(RegistryError::BadId {
                    id: reg.id,
                    source: reg.source,
                });
            }
            if let Some(first) = by_id.insert(reg.id, reg) {
                return Err(RegistryError::DuplicateId {
                    id: reg.id,
                    first: first.source,
                    second: reg.source,
                });
            }
            let function = FUNCTIONS.iter().find(|f| f.name == reg.sakura).ok_or(
                RegistryError::UnknownSakura {
                    id: reg.id,
                    sakura: reg.sakura,
                    source: reg.source,
                },
            )?;
            if !matches!(
                function.kind,
                FunctionKind::Command | FunctionKind::MacroOnly | FunctionKind::Internal
            ) {
                return Err(RegistryError::NotACommand {
                    id: reg.id,
                    sakura: reg.sakura,
                    source: reg.source,
                });
            }
            if let Some(first) = by_code.insert(function.code, reg) {
                return Err(RegistryError::DuplicateSakura {
                    sakura: reg.sakura,
                    first: first.id,
                    second: reg.id,
                });
            }
        }
        Ok(Self { by_id, by_code })
    }

    /// The registry of the commands linked into this program. Built on first use; panics if the
    /// registered commands are wrong (the tests of this crate catch that before it ships).
    pub fn global() -> &'static Self {
        static REGISTRY: OnceLock<Registry> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            Self::build().unwrap_or_else(|e| panic!("the registered commands are wrong: {e}"))
        })
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// All commands, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = &'static Registration> + '_ {
        self.by_id.values().copied()
    }

    pub fn get(&self, id: &str) -> Option<&'static Registration> {
        self.by_id.get(id).copied()
    }

    /// The command that implements Sakura's function `code` (`FUNCTIONS[..].code`).
    pub fn get_by_code(&self, code: u16) -> Option<&'static Registration> {
        self.by_code.get(&code).copied()
    }

    /// Runs the command `id` as one undo block.
    pub fn run(&self, id: &str, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        self.get(id)
            .ok_or_else(|| CommandError::Unknown(id.to_owned()))?
            .run(cx, args)
    }

    /// Runs the command for Sakura's function `code` as one undo block.
    pub fn run_code(&self, code: u16, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        self.get_by_code(code)
            .ok_or_else(|| CommandError::Unknown(format!("function code {code}")))?
            .run(cx, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_is_not_empty() {
        // `inventory` registers nothing, silently, where it does not work (ADR-0013).
        assert!(!Registry::global().is_empty());
    }

    #[test]
    fn the_registered_commands_pass_the_checks() {
        // IDs and Sakura functions are unique, and every function is in Sakura's table.
        let registry = Registry::build().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            registry.len(),
            inventory::iter::<Registration>.into_iter().count()
        );
    }

    #[test]
    fn lookup_by_id_and_by_code_agree() {
        let registry = Registry::global();
        for reg in registry.iter() {
            let function = FUNCTIONS.iter().find(|f| f.name == reg.sakura).unwrap();
            assert_eq!(registry.get(reg.id).unwrap().id, reg.id);
            assert_eq!(registry.get_by_code(function.code).unwrap().id, reg.id);
        }
        assert!(registry.get("no_such_command").is_none());
        assert!(registry.get_by_code(0).is_none());
    }

    #[test]
    fn a_command_file_is_named_after_its_id() {
        // The convention that makes a command easy to find: `commands/<group>/<id>.rs`.
        for reg in Registry::global().iter() {
            let stem = std::path::Path::new(reg.source)
                .file_stem()
                .and_then(|s| s.to_str());
            assert_eq!(stem, Some(reg.id), "{}", reg.source);
        }
    }

    #[test]
    fn every_function_code_found_by_name_is_the_one_in_the_table() {
        let registry = Registry::global();
        let f = FUNCTIONS
            .iter()
            .find(|f| f.name == "F_DELETE_BACK")
            .unwrap();
        assert_eq!(registry.get_by_code(f.code).unwrap().id, "delete_back");
    }

    /// Commands that are never registered, to feed the checks with mistakes.
    macro_rules! dummy {
        ($name:ident, $id:expr, $sakura:expr) => {
            #[derive(Default)]
            struct $name;
            impl crate::command::Command for $name {
                const ID: &'static str = $id;
                const SAKURA: &'static str = $sakura;
                fn run(&self, _: &mut CommandContext<'_>, _: &[Arg]) -> Result {
                    Ok(())
                }
            }
        };
    }

    fn build(regs: &'static [Registration]) -> std::result::Result<Registry, RegistryError> {
        Registry::from_registrations(regs)
    }

    #[test]
    fn mistakes_are_reported() {
        dummy!(A, "a", "F_DELETE");
        dummy!(A2, "a", "F_UNDO");
        dummy!(B, "b", "F_DELETE");
        dummy!(C, "c", "F_NO_SUCH_FUNCTION");
        dummy!(D, "d", "F_0");
        dummy!(E, "Bad-Id", "F_REDO");
        static OK: [Registration; 1] = [Registration::of::<A>("a.rs")];
        assert!(build(&OK).is_ok());
        static DUP_ID: [Registration; 2] = [
            Registration::of::<A>("a.rs"),
            Registration::of::<A2>("a2.rs"),
        ];
        assert!(matches!(
            build(&DUP_ID),
            Err(RegistryError::DuplicateId { id: "a", .. })
        ));
        static DUP_SAKURA: [Registration; 2] =
            [Registration::of::<A>("a.rs"), Registration::of::<B>("b.rs")];
        assert!(matches!(
            build(&DUP_SAKURA),
            Err(RegistryError::DuplicateSakura {
                sakura: "F_DELETE",
                ..
            })
        ));
        static UNKNOWN: [Registration; 1] = [Registration::of::<C>("c.rs")];
        assert!(matches!(
            build(&UNKNOWN),
            Err(RegistryError::UnknownSakura { .. })
        ));
        static BAD_KIND: [Registration; 1] = [Registration::of::<D>("d.rs")];
        assert!(matches!(
            build(&BAD_KIND),
            Err(RegistryError::NotACommand { .. })
        ));
        static BAD_ID: [Registration; 1] = [Registration::of::<E>("e.rs")];
        assert!(matches!(build(&BAD_ID), Err(RegistryError::BadId { .. })));
    }

    #[test]
    fn snake_case_ids() {
        assert!(is_snake_case("file_save"));
        assert!(is_snake_case("go_line_top_sel"));
        assert!(is_snake_case("insert_2"));
        for bad in ["", "File", "a-b", "_a", "a_", "a__b", "2a", "a b"] {
            assert!(!is_snake_case(bad), "{bad:?}");
        }
    }
}
