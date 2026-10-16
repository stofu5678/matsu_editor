//! `F_DIFF`: marks the lines that differ from a file, without the dialog (DIFF差分表示).
//!
//! Sakura: `CViewCommander::Command_Diff` (`sakura_core/cmd/CViewCommander_Diff.cpp`), macro
//! `Diff(file, flags)` (`CMacro::HandleCommand`). The file is required (Sakura's macro stops with
//! an error message without it). The flags are the sum of 0x0001 ignore case, 0x0002 ignore all
//! white space, 0x0004 ignore changes in white space, 0x0008 ignore blank lines, 0x0010 expand
//! tabs, 0x0020 this window is the old file and 0x0040 say so when there is no difference; none
//! if left out. The work is `EditorView::run_diff`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, CommandError, Result};

use crate::diff::Target;
use crate::file_ops::args::path_arg;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Diff;

/// The flags of argument `i` (`wtoi_def( Argument[1], 0 )`: a number, or text that is one).
fn flags_arg(args: &[Arg], i: usize) -> std::result::Result<u32, CommandError> {
    let value = match args.get(i) {
        None => 0,
        Some(Arg::Int(n)) => *n,
        Some(Arg::Text(t)) => t.trim().parse().unwrap_or(0),
        Some(other) => {
            return Err(CommandError::BadArgs(format!(
                "argument {} must be a number, not {other:?}",
                i + 1
            )));
        }
    };
    Ok(u32::try_from(value).unwrap_or(0))
}

impl UiCommand for Diff {
    const ID: &'static str = "diff";
    const SAKURA: &'static str = "F_DIFF";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let path = path_arg(args, 0)?.ok_or_else(|| {
            CommandError::BadArgs("Diff needs the file to compare with".to_owned())
        })?;
        let flags = flags_arg(args, 1)?;
        view.run_diff(Target::File(path), flags, window, cx);
        Ok(())
    }
}

register_ui_command!(Diff);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flags_are_a_number_or_text_or_nothing() {
        assert_eq!(flags_arg(&[], 1).unwrap(), 0);
        let file = Arg::Text("a.txt".to_owned());
        assert_eq!(flags_arg(&[file.clone(), Arg::Int(33)], 1).unwrap(), 33);
        assert_eq!(
            flags_arg(&[file.clone(), Arg::Text(" 9 ".to_owned())], 1).unwrap(),
            9
        );
        assert_eq!(
            flags_arg(&[file.clone(), Arg::Text("x".to_owned())], 1).unwrap(),
            0
        );
        assert_eq!(flags_arg(&[file.clone(), Arg::Int(-1)], 1).unwrap(), 0);
        assert!(flags_arg(&[file, Arg::Char('x')], 1).is_err());
    }
}
