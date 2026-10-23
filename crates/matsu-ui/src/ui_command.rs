//! Commands that need the window.
//!
//! Most of Sakura's functions are the commands of `matsu-commands`, which know nothing about GPUI
//! (ADR-0013). A few need what only the window has: the size of the view (scrolling, page up and
//! down), a dialog (open, save as, find), the clipboard of the OS, another window. They are written
//! here in the same way as the core commands: one command per file,
//! `ui_commands/<group>/<id>.rs`, registered with `register_ui_command!`, and found by ID or by
//! Sakura's function code. Nothing lists them.
//!
//! A key, a menu item or a macro names a Sakura function, and
//! [`EditorView::run_function`] runs the core command for it if there is one, else the UI command.
//! A function is implemented in one place only; [`UiRegistry::build`] checks that.
//!
//! A UI command that changes the text does it through `EditorView::run_with` (or by running a
//! core command), so that the layout, the undo history and the caret follow as for any command.

use std::collections::HashMap;
use std::sync::OnceLock;

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_commands::registry::Registry;
use matsu_commands::sakura::{FUNCTIONS, FunctionKind};

use crate::view::EditorView;

/// A command that runs on the view, with GPUI's window and context.
pub trait UiCommand: Default {
    /// A readable lowercase `snake_case` ID, unique among the core and the UI commands.
    const ID: &'static str;
    /// The Sakura function it implements: a name in Sakura's function table (`F_CURLINECENTER`).
    const SAKURA: &'static str;

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result;
}

type RunFn = fn(&mut EditorView, &[Arg], &mut Window, &mut Context<EditorView>) -> Result;

fn run_default<C: UiCommand>(
    view: &mut EditorView,
    args: &[Arg],
    window: &mut Window,
    cx: &mut Context<EditorView>,
) -> Result {
    C::default().run(view, args, window, cx)
}

/// What `register_ui_command!` submits for a UI command.
#[derive(Debug)]
pub struct UiRegistration {
    pub id: &'static str,
    pub sakura: &'static str,
    /// The file that registered the command, for messages about mistakes.
    pub source: &'static str,
    func: RunFn,
}

impl UiRegistration {
    #[doc(hidden)]
    pub const fn of<C: UiCommand>(source: &'static str) -> Self {
        Self {
            id: C::ID,
            sakura: C::SAKURA,
            source,
            func: run_default::<C>,
        }
    }

    pub fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        (self.func)(view, args, window, cx)
    }
}

inventory::collect!(UiRegistration);

/// Registers a UI command: `register_ui_command!(CurLineCenter);` in the command's file.
macro_rules! register_ui_command {
    ($command:ty) => {
        inventory::submit! {
            $crate::ui_command::UiRegistration::of::<$command>(file!())
        }
    };
}
pub(crate) use register_ui_command;

/// All UI commands of this program.
#[derive(Debug)]
pub struct UiRegistry {
    by_id: HashMap<&'static str, &'static UiRegistration>,
    by_code: HashMap<u16, &'static UiRegistration>,
}

fn is_snake_case(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with(|c: char| c == '_' || c.is_ascii_digit())
        && !id.ends_with('_')
        && !id.contains("__")
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl UiRegistry {
    /// Collects and checks the UI commands: the same checks as the core registry, and no ID or
    /// function that is also a core command.
    pub fn build() -> std::result::Result<Self, String> {
        let core = Registry::global();
        let mut by_id: HashMap<&'static str, &'static UiRegistration> = HashMap::new();
        let mut by_code: HashMap<u16, &'static UiRegistration> = HashMap::new();
        for reg in inventory::iter::<UiRegistration> {
            let (id, source) = (reg.id, reg.source);
            if !is_snake_case(id) {
                return Err(format!(
                    "UI command ID {id:?} ({source}) is not lowercase snake_case"
                ));
            }
            if core.get(id).is_some() {
                return Err(format!(
                    "UI command ID {id:?} ({source}) is also a core command"
                ));
            }
            if let Some(first) = by_id.insert(id, reg) {
                return Err(format!(
                    "UI command ID {id:?} is registered twice ({} and {source})",
                    first.source
                ));
            }
            let function = FUNCTIONS
                .iter()
                .find(|f| f.name == reg.sakura)
                .ok_or_else(|| {
                    format!(
                        "UI command {id:?} ({source}) names {:?}, which is not in Sakura's function table",
                        reg.sakura
                    )
                })?;
            if !matches!(
                function.kind,
                FunctionKind::Command | FunctionKind::MacroOnly | FunctionKind::Internal
            ) {
                return Err(format!(
                    "UI command {id:?} ({source}) names {:?}, which is not a function that can be run",
                    reg.sakura
                ));
            }
            if let Some(other) = core.get_by_code(function.code) {
                return Err(format!(
                    "Sakura's {} is both the core command {:?} and the UI command {id:?}",
                    reg.sakura, other.id
                ));
            }
            if let Some(first) = by_code.insert(function.code, reg) {
                return Err(format!(
                    "Sakura's {} is implemented twice (UI commands {:?} and {id:?})",
                    reg.sakura, first.id
                ));
            }
        }
        Ok(Self { by_id, by_code })
    }

    /// The UI commands linked into this program. Built on first use; panics if they are wrong
    /// (the tests catch that before it ships).
    pub fn global() -> &'static Self {
        static REGISTRY: OnceLock<UiRegistry> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            Self::build().unwrap_or_else(|e| panic!("the registered UI commands are wrong: {e}"))
        })
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// All UI commands, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = &'static UiRegistration> + '_ {
        self.by_id.values().copied()
    }

    pub fn get(&self, id: &str) -> Option<&'static UiRegistration> {
        self.by_id.get(id).copied()
    }

    /// The UI command that implements Sakura's function `code`.
    pub fn get_by_code(&self, code: u16) -> Option<&'static UiRegistration> {
        self.by_code.get(&code).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ui_commands_pass_the_checks() {
        let registry = UiRegistry::build().unwrap_or_else(|e| panic!("{e}"));
        assert!(!registry.is_empty(), "inventory registered no UI command");
        assert_eq!(
            registry.len(),
            inventory::iter::<UiRegistration>.into_iter().count()
        );
    }

    #[test]
    fn lookup_by_id_and_by_code_agree() {
        let registry = UiRegistry::global();
        for reg in registry.iter() {
            let function = FUNCTIONS.iter().find(|f| f.name == reg.sakura).unwrap();
            assert_eq!(registry.get(reg.id).unwrap().id, reg.id);
            assert_eq!(registry.get_by_code(function.code).unwrap().id, reg.id);
        }
        assert!(registry.get("no_such_command").is_none());
    }

    #[test]
    fn a_ui_command_file_is_named_after_its_id() {
        for reg in UiRegistry::global().iter() {
            let stem = std::path::Path::new(reg.source)
                .file_stem()
                .and_then(|s| s.to_str());
            assert_eq!(stem, Some(reg.id), "{} registers {:?}", reg.source, reg.id);
        }
    }
}
