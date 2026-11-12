//! The shell tab (ADR-0028): a terminal in a tab, the second kind of tab that is not an editor.
//!
//! * [`view`]: the view, with the keys, the events of the shell and the size.
//! * `element`: the drawing of the screen (every cell on its column).
//! * `input`: what a key press means to the terminal.
//! * `palette`: the colors.
//! * [`menus`]: the menus of the window while a terminal is active (`MenuKind::Terminal`).
//!
//! The tab is opened by `F_SHELL_TAB` (MATSU Editor's own function, `ui_commands/window/shell_tab.rs`), and the
//! window opens and closes it in `tabs/terminal_tab.rs`. The terminal itself, with no UI, is `matsu-terminal`.
//!
//! Like the preview, the tab keeps a view of an empty document out of sight, so that the window always has a
//! view to ask (`TabContent::panes`); it is not one of the window's editors. Nothing can be sent to a terminal
//! except the user's keys: no macro or plugin function writes to it (ADR-0028 point 2), and opening the tab
//! starts the shell and sends nothing (point 3).

mod element;
mod input;
pub(crate) mod menus;
mod palette;
mod view;

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{
    App, AppContext as _, AsyncWindowContext, Context, Entity, FocusHandle, WeakEntity, Window,
    actions,
};
use matsu_terminal::{Options, Rgb, ShellCommand, Terminal, TerminalSize};

use crate::app::Workspace;
use crate::panes::PaneSet;
use crate::status::StatusInfo;
use crate::tabs::content::{CloseCheck, MenuKind, TabPanel};
use crate::view::text_font::TextFont;
use palette::Palette;
use view::Look;
pub(crate) use view::TerminalView;
#[cfg(test)]
pub(crate) use view::is_window_function;

actions!(
    matsu_terminal_tab,
    [
        /// Closes the shell tab (asking first when the shell is running).
        Close
    ]
);

/// How long the end of the editor waits for the closing shells (`matsu_terminal::wait_for_closing`).
///
/// Windows ends a program that has not answered the end of the session after about five seconds, and the
/// copies of the unsaved documents wait up to three of them (`unsaved::settle_at_quit`). The shells are closed
/// first, at the same time as the copies are written, so by the time the copies are done they have had that
/// long already; this is the time added after it. Three plus one is four seconds, which leaves the rest of the
/// end a second. A shell that is not closed by then is one that hangs (ConPTY waiting for a client): waiting
/// longer would not help, and the end must not hang with it.
pub(crate) const QUIT_WAIT: Duration = Duration::from_secs(1);

/// The size a shell starts with, until the first paint tells the real one.
const START_SIZE: TerminalSize = TerminalSize::new(80, 24);

/// What a new shell tab needs to know about the window it opens in.
pub(crate) struct Start {
    /// `None` is the default shell (`matsu_terminal::shell::default_shell`); only the tests give another.
    pub shell: Option<ShellCommand>,
    /// The file of the active document: the shell starts in its folder.
    pub document_path: Option<PathBuf>,
    /// The editor's text font and size, and its default colors (`0xRRGGBB`).
    pub font: TextFont,
    pub font_size: f32,
    pub foreground: u32,
    pub background: u32,
}

fn rgb_of(color: u32) -> Rgb {
    Rgb {
        r: (color >> 16) as u8,
        g: (color >> 8) as u8,
        b: color as u8,
    }
}

/// Starts a shell and makes its view. The shell starts in the folder of the document, else in the home
/// folder; a folder that is not one (the file's folder was removed) is replaced by the home folder too.
pub(crate) fn start(
    start: Start,
    host: WeakEntity<Workspace>,
    window: &Window,
    cx: &mut Context<Workspace>,
) -> io::Result<Entity<TerminalView>> {
    let shell = resolve_shell(start.shell.clone());
    let mut directory = start_directory(start.document_path.as_deref());
    let mut options = Options {
        size: START_SIZE,
        working_directory: directory.clone(),
        shell: shell.clone(),
        foreground: rgb_of(start.foreground),
        background: rgb_of(start.background),
        ..Options::default()
    };
    let (terminal, events) = match Terminal::spawn(options.clone()) {
        Err(error)
            if directory.is_some()
                && matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
        {
            directory = home_dir();
            options.working_directory.clone_from(&directory);
            Terminal::spawn(options)?
        }
        other => other?,
    };
    let program = program_name(shell.as_ref());
    let tooltip = view::tooltip_text(&program, directory.as_ref());
    let look = Look {
        font: start.font,
        size: start.font_size,
        palette: Palette::new(start.foreground, start.background),
    };
    let view =
        cx.new(|cx| TerminalView::new(terminal, START_SIZE, look, (program, tooltip), host, cx));
    view.update(cx, |view, cx| view.start_events(events, window, cx));
    Ok(view)
}

/// The running shells of all windows, by the entity of their views.
pub(crate) fn running_shell_ids(cx: &App) -> Vec<gpui::EntityId> {
    crate::file_ops::windows::listed_workspaces(cx)
        .iter()
        .flat_map(|(_, workspace)| workspace.read(cx).running_shell_ids(cx))
        .collect()
}

/// Ends every shell now, for the end of the editor. The closing runs on other threads, which
/// `matsu_terminal::wait_for_closing` waits for.
pub(crate) fn shutdown_all(cx: &mut App) {
    let views: Vec<_> = crate::file_ops::windows::listed_workspaces(cx)
        .iter()
        .flat_map(|(_, workspace)| workspace.read(cx).shell_views())
        .collect();
    for view in views {
        view.update(cx, |view, _| view.shutdown());
    }
}

/// The question before running shells are ended by closing a tab, a window or the editor: one question for
/// all of them. `true` means close.
pub(crate) async fn confirm_close(cx: &mut AsyncWindowContext, shells: usize) -> bool {
    let message = if shells > 1 {
        format!(
            "シェルが {shells} 個動いています。閉じると、シェルとその中で動いているプログラムが終わります。閉じますか?"
        )
    } else {
        "シェルが動いています。閉じると、シェルとその中で動いているプログラムが終わります。閉じますか?"
            .to_owned()
    };
    crate::file_ops::ask::confirm(cx, &message, None, ["はい(Y)", "いいえ(N)"]).await
}

/// The keys of the terminal. Its key context is "Terminal".
pub(crate) fn bind_keys(cx: &mut App) {
    // gpui-component's `Root` binds Tab and Shift+Tab to "move the focus", and GPUI runs key bindings before
    // the key listeners of the views (see `EditorView::bind_keys`): the terminal needs them as keys.
    cx.bind_keys([
        gpui::KeyBinding::new("tab", gpui::NoAction, Some("Terminal")),
        gpui::KeyBinding::new("shift-tab", gpui::NoAction, Some("Terminal")),
    ]);
    // Ctrl+W is the program's (delete the word before the cursor); Cmd+W closes the tab on macOS.
    if cfg!(target_os = "macos") {
        cx.bind_keys([gpui::KeyBinding::new("cmd-w", Close, Some("Terminal"))]);
    }
}

/// A shell tab.
pub(crate) struct TerminalTab {
    /// A view of an empty document, never drawn (see the module).
    panes: PaneSet,
    view: Entity<TerminalView>,
}

impl TerminalTab {
    pub(crate) fn new(panes: PaneSet, view: Entity<TerminalView>) -> Self {
        Self { panes, view }
    }

    pub(crate) fn panes(&self) -> &PaneSet {
        &self.panes
    }

    pub(crate) fn panes_mut(&mut self) -> &mut PaneSet {
        &mut self.panes
    }

    pub(crate) fn view(&self) -> &Entity<TerminalView> {
        &self.view
    }
}

impl TabPanel for TerminalTab {
    /// The shell's program name (`pwsh`, `zsh`); the program's own title is a later PR. A shell that ended
    /// says so.
    fn caption(&self, cx: &App) -> String {
        let view = self.view.read(cx);
        match view.ended() {
            Some(_) => format!("{} (終了)", view.program()),
            None => view.program().to_owned(),
        }
    }

    fn tooltip(&self, cx: &App) -> String {
        self.view.read(cx).tooltip().to_owned()
    }

    fn icon(&self, _: &App) -> crate::tabs::icon::TabIcon {
        crate::tabs::icon::TabIcon::Terminal
    }

    fn accessible_name(&self, cx: &App) -> String {
        format!("シェル: {}", self.caption(cx))
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        gpui::Focusable::focus_handle(self.view.read(cx), cx)
    }

    fn menu_kind(&self) -> MenuKind {
        MenuKind::Terminal
    }

    fn status(&self, cx: &App) -> Option<StatusInfo> {
        Some(self.view.read(cx).status())
    }

    /// A running shell is asked about; one that ended is not.
    fn close_check(&self, cx: &App) -> CloseCheck {
        if self.view.read(cx).is_running() {
            CloseCheck::RunningProcess
        } else {
            CloseCheck::None
        }
    }
}

/// The shell to start: the one given, else the default of `matsu-terminal` (PowerShell on Windows). On Unix the
/// default is left to `alacritty_terminal` (`None`), whose choice is `$SHELL`, which `program_name` reads too.
/// Resolved here so that the caption names the shell that really starts.
fn resolve_shell(given: Option<ShellCommand>) -> Option<ShellCommand> {
    given.or_else(matsu_terminal::shell::default_shell)
}

/// The name of a shell's program as the caption shows it: the file name without its folder and without
/// `.exe`. `None` is the default shell of Unix, which is `$SHELL`.
pub(crate) fn program_name(shell: Option<&ShellCommand>) -> String {
    let from_env;
    let program = match shell {
        Some(shell) => shell.program.as_str(),
        None => {
            from_env = std::env::var("SHELL").unwrap_or_default();
            from_env.as_str()
        }
    };
    let program = program.trim().trim_matches('"');
    let file = program.rsplit(['/', '\\']).next().unwrap_or(program);
    let name = match file.len().checked_sub(4) {
        Some(at) if file.is_char_boundary(at) && file[at..].eq_ignore_ascii_case(".exe") => {
            &file[..at]
        }
        _ => file,
    };
    if name.is_empty() {
        "シェル".to_owned()
    } else {
        name.to_owned()
    }
}

/// The home folder, where a shell starts when the document has no folder.
pub(crate) fn home_dir() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    var("HOME")
        .or_else(|| var("USERPROFILE"))
        .or_else(|| {
            let (drive, path) = (var("HOMEDRIVE")?, var("HOMEPATH")?);
            let mut home = drive;
            home.push(path);
            Some(home)
        })
        .map(PathBuf::from)
}

/// The folder a shell starts in for a document: the folder of its file, else the home folder.
pub(crate) fn start_directory(document_path: Option<&Path>) -> Option<PathBuf> {
    document_path
        .and_then(Path::parent)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .or_else(home_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_caption_is_the_program_without_folder_and_extension() {
        let shell = |program: &str| ShellCommand::new(program, Vec::<String>::new());
        assert_eq!(
            program_name(Some(&shell(r"C:\Program Files\PowerShell\7\pwsh.exe"))),
            "pwsh"
        );
        assert_eq!(program_name(Some(&shell("/bin/zsh"))), "zsh");
        assert_eq!(program_name(Some(&shell("cmd.EXE"))), "cmd");
        assert_eq!(program_name(Some(&shell("\"C:\\a b\\fish.exe\""))), "fish");
        assert_eq!(program_name(Some(&shell(""))), "シェル");
        assert_eq!(program_name(Some(&shell(".exe"))), "シェル");
    }

    #[test]
    fn the_caption_names_the_default_shell_that_starts() {
        let given = ShellCommand::new("/bin/zsh", Vec::<String>::new());
        assert_eq!(resolve_shell(Some(given.clone())), Some(given));
        let resolved = resolve_shell(None);
        if cfg!(windows) {
            let name = program_name(resolved.as_ref());
            assert!(name == "pwsh" || name == "powershell", "{name}");
        } else {
            assert_eq!(resolved, None);
            assert_eq!(program_name(resolved.as_ref()), program_name(None));
        }
    }

    #[test]
    fn a_document_starts_the_shell_in_its_folder_and_one_without_a_file_at_home() {
        assert_eq!(
            start_directory(Some(Path::new("/work/notes/a.txt"))),
            Some(PathBuf::from("/work/notes"))
        );
        // a bare file name has no folder to go to
        assert_eq!(start_directory(Some(Path::new("a.txt"))), home_dir());
        assert_eq!(start_directory(None), home_dir());
    }
}
