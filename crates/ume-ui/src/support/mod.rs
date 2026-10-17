//! Sakura's support group (支援): the input completion list, the command list and the version
//! information. They are small windows over the editor view; the text area's own code only calls
//! the few hooks named here.
//!
//! * [`completion`]: the list at the caret (`F_HOKAN`). It is not a window of its own: the view
//!   keeps the focus and the keys, as Sakura's completion window never takes the focus.
//! * [`command_list`]: the menu of all functions (`F_MENU_ALLFUNC`). A modal list with the focus.
//! * [`about`]: the version dialog (`F_ABOUT`). A modal dialog with the focus.
//!
//! The view holds one [`Support`] and draws [`EditorView::support_overlays`] over the text.

use gpui::{AnyElement, Context, Entity, IntoElement as _};

use crate::view::EditorView;

pub(crate) mod about;
pub(crate) mod command_list;
pub(crate) mod completion;

/// What the view shows of the support group.
#[derive(Default)]
pub(crate) struct Support {
    /// The completion list, while it is shown.
    pub completion: Option<completion::Completion>,
    /// The command list, while it is open.
    pub command_list: Option<Entity<command_list::CommandList>>,
    /// The version dialog, while it is open.
    pub about: Option<Entity<about::AboutDialog>>,
    /// The tag jump list, while it is open (`crate::tags`).
    pub tag_list: Option<Entity<crate::tags::list::TagList>>,
    /// The dialog of 「タグファイルの作成」, while it is open.
    pub tags_make: Option<Entity<crate::tags::make::TagsMakeDialog>>,
    /// The dialog of 外部コマンド実行, while it is open (`crate::exec`).
    pub exec_dialog: Option<Entity<crate::exec::dialog::ExecDialog>>,
    /// The box of a command that runs, with its 中断 button.
    pub exec_running: Option<Entity<crate::exec::running::ExecRunning>>,
}

impl EditorView {
    /// The elements to draw over the view, in order.
    pub(crate) fn support_overlays(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut out = Vec::new();
        out.extend(self.render_completion(cx));
        out.extend(
            self.support
                .command_list
                .clone()
                .map(|e| e.into_any_element()),
        );
        out.extend(self.support.about.clone().map(|e| e.into_any_element()));
        out.extend(self.support.tag_list.clone().map(|e| e.into_any_element()));
        out.extend(self.support.tags_make.clone().map(|e| e.into_any_element()));
        out.extend(
            self.support
                .exec_dialog
                .clone()
                .map(|e| e.into_any_element()),
        );
        out.extend(
            self.support
                .exec_running
                .clone()
                .map(|e| e.into_any_element()),
        );
        out
    }
}

#[cfg(test)]
mod tests;
