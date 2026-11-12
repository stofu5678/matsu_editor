//! What the window does with its shell tabs (ADR-0028, `crate::terminal`): opens one, closes one (asking
//! first when the shell is running), and counts the running shells for the questions that end a window or
//! the editor.

use gpui::{AppContext as _, Context, Entity, Window};

use super::content::TabContent;
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::panes::PaneSet;
use crate::terminal::{self, TerminalTab, TerminalView};
use crate::view::EditorView;

#[cfg(test)]
thread_local! {
    /// The shell that `F_SHELL_TAB` starts in a test, in place of the user's.
    static SHELL_FOR_TESTS: std::cell::RefCell<Option<matsu_terminal::ShellCommand>> =
        const { std::cell::RefCell::new(None) };
}

/// Makes `F_SHELL_TAB` start `shell` in this test: the tests start a program of their own, never the user's
/// shell.
#[cfg(test)]
pub(crate) fn use_shell_in_tests(shell: matsu_terminal::ShellCommand) {
    SHELL_FOR_TESTS.with(|slot| *slot.borrow_mut() = Some(shell));
}

/// The shell a test asked for, if it did.
fn shell_for_tests() -> Option<matsu_terminal::ShellCommand> {
    #[cfg(test)]
    {
        SHELL_FOR_TESTS.with(|slot| slot.borrow().clone())
    }
    #[cfg(not(test))]
    {
        None
    }
}

impl Workspace {
    /// `F_SHELL_TAB`: opens a shell in a tab next to the active one. Every call opens another shell.
    pub(crate) fn open_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_terminal(shell_for_tests(), window, cx);
    }

    /// [`open_shell`](Self::open_shell) with a command in place of the user's shell: the tests start their own
    /// program, not the shell of the machine they run on.
    #[cfg(test)]
    pub(crate) fn open_shell_with(
        &mut self,
        shell: matsu_terminal::ShellCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<TerminalView>> {
        self.open_terminal(Some(shell), window, cx)
    }

    fn open_terminal(
        &mut self,
        shell: Option<matsu_terminal::ShellCommand>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<TerminalView>> {
        let editor = self.panes().active().clone();
        let (document_path, font, font_size, colors) = {
            let view = editor.read(cx);
            (
                view.document()
                    .read(cx)
                    .path()
                    .map(std::path::Path::to_path_buf),
                view.text_font(cx),
                view.font_px(),
                view.view_settings().colors,
            )
        };
        let host = cx.weak_entity();
        let started = terminal::start(
            terminal::Start {
                shell,
                document_path,
                font,
                font_size,
                foreground: colors.text,
                background: colors.background,
            },
            host.clone(),
            window,
            cx,
        );
        let view = match started {
            Ok(view) => view,
            Err(error) => {
                let message = format!("シェルを起動できません: {error}");
                if self.content.is_editor() {
                    // The status bar of the editor that is shown.
                    editor.update(cx, |view, cx| {
                        view.notice = Some(message);
                        cx.notify();
                    });
                } else {
                    // The view of a tab that is not an editor is not drawn: a box says it.
                    window
                        .spawn(cx, async move |cx| {
                            crate::file_ops::ask::tell(
                                cx,
                                gpui::PromptLevel::Warning,
                                &message,
                                None,
                            )
                            .await;
                        })
                        .detach();
                }
                return None;
            }
        };
        // The hidden view of an empty document, so that the window always has a view to ask.
        let document = cx.new(|_| SharedDocument::empty());
        let hidden = cx.new(|cx| EditorView::new(document, cx));
        hidden.update(cx, |view, _| view.split_host = Some(host));
        let tab = TabContent::Terminal(TerminalTab::new(PaneSet::new(hidden), view.clone()));
        let at = self.active_tab() + 1;
        self.insert_content(tab, at, true, window, cx);
        Some(view)
    }

    /// The tab that has `view`.
    pub(crate) fn tab_of_shell(&self, view: &Entity<TerminalView>) -> Option<usize> {
        (0..self.tab_count()).find(|&i| {
            self.tab_content(i)
                .and_then(TabContent::terminal)
                .is_some_and(|tab| tab.view() == view)
        })
    }

    /// The views of the shells of this window.
    pub(crate) fn shell_views(&self) -> Vec<Entity<TerminalView>> {
        (0..self.tab_count())
            .filter_map(|i| self.tab_content(i)?.terminal())
            .map(|tab| tab.view().clone())
            .collect()
    }

    /// Which shells of this window are running: the entities of their views.
    pub(crate) fn running_shell_ids(&self, cx: &gpui::App) -> Vec<gpui::EntityId> {
        self.shell_views()
            .iter()
            .filter(|view| view.read(cx).is_running())
            .map(Entity::entity_id)
            .collect()
    }

    /// Whether a running shell of this window is not among `confirmed`: one the user was not asked about.
    pub(crate) fn has_unconfirmed_shell(
        &self,
        cx: &gpui::App,
        confirmed: &[gpui::EntityId],
    ) -> bool {
        self.running_shell_ids(cx)
            .iter()
            .any(|id| !confirmed.contains(id))
    }

    /// How many shells of this window are running.
    pub(crate) fn running_shells(&self, cx: &gpui::App) -> usize {
        self.shell_views()
            .iter()
            .filter(|view| view.read(cx).is_running())
            .count()
    }

    /// Closes the tab of `view` without a question, or the window when it is the only tab.
    pub(crate) fn close_shell_now(
        &mut self,
        view: &Entity<TerminalView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.tab_of_shell(view) {
            self.close_tab_or_window(index, window, cx);
        }
    }

    /// Closes tab `index`, which is not an editor: a preview just goes; a running shell is asked about
    /// first (ADR-0028 point 10), and one that has ended goes at once.
    pub(crate) fn close_own_tab(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shell = self
            .tab_content(index)
            .and_then(TabContent::terminal)
            .map(|tab| tab.view().clone());
        let Some(shell) = shell.filter(|view| view.read(cx).is_running()) else {
            self.close_tab_or_window(index, window, cx);
            return;
        };
        let workspace = cx.weak_entity();
        window
            .spawn(cx, async move |cx| {
                if !terminal::confirm_close(cx, 1).await {
                    return;
                }
                cx.update(|window, app| {
                    workspace
                        .update(app, |ws, cx| ws.close_shell_now(&shell, window, cx))
                        .ok();
                })
                .ok();
            })
            .detach();
    }
}
