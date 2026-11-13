//! The settings of the shell tab as the windows hold them (`[Terminal]` of `matsu.ini`, ADR-0028 point 11).
//!
//! The page シェル of the common settings sets them when the application starts and at every OK
//! (`common_settings::pages::terminal`). A shell tab that starts reads them then: the shell and the length of its
//! scrollback are those of the moment it starts, and a tab that is open keeps them. "Option as Meta" is read
//! at every key, so it changes in the tabs that are open too.

use gpui::{App, Global};
use matsu_config::terminal::TerminalSettings;
use matsu_terminal::ShellCommand;

/// The settings now.
struct Current(TerminalSettings);

impl Global for Current {}

/// The settings now (the defaults until the application has read the file).
pub(crate) fn current(cx: &App) -> TerminalSettings {
    cx.try_global::<Current>()
        .map(|current| current.0.clone())
        .unwrap_or_default()
}

/// Takes `settings` as the ones that new shell tabs start with.
pub(crate) fn set(cx: &mut App, settings: TerminalSettings) {
    cx.set_global(Current(settings));
}

/// What the shell of the settings is: `None` for the default shell (the setting is empty), or when the command
/// line cannot be used (a quote that is not closed): the tab starts with the default shell then, as the other
/// values that cannot be used are the default.
pub(crate) fn shell_of(settings: &TerminalSettings) -> Option<ShellCommand> {
    if settings.shell.is_empty() {
        return None;
    }
    ShellCommand::from_command_line(&settings.shell).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_shell(shell: &str) -> TerminalSettings {
        TerminalSettings::from_values(shell, 5000, false)
    }

    #[test]
    fn an_empty_or_unusable_shell_is_the_default() {
        assert_eq!(shell_of(&with_shell("")), None);
        assert_eq!(shell_of(&with_shell("   ")), None);
        assert_eq!(shell_of(&with_shell("\"sh -c x")), None);
    }

    #[test]
    fn the_command_line_is_the_shell() {
        let shell = shell_of(&with_shell("sh -l")).expect("a shell");
        assert_eq!(shell.program, "sh");
        if cfg!(windows) {
            assert_eq!(shell.raw_arguments.as_deref(), Some("-l"));
        } else {
            assert_eq!(shell.args, ["-l"]);
        }
    }
}
