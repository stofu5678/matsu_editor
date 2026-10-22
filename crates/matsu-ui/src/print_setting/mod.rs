//! 印刷ページ設定 (`F_PRINT_PAGESETUP`): the eight print settings, the dialog that edits them, and
//! what the preview and printing take from them.
//!
//! Sakura: `CViewCommander::Command_PRINT_PAGESETUP` calls `CEditWnd::OnPrintPageSetting`
//! (`sakura_core/window/CEditWnd.cpp`), which shows `CDlgPrintSetting`
//! (`sakura_core/dlg/CDlgPrintSetting.cpp`). OK stores the eight settings in the shared data and
//! the number of the chosen one in the settings of the document type, and tells every editor
//! window (`MYWM_CHANGESETTING` with `PM_PRINTSETTING`), whose print previews lay the document out
//! again (`CPrintPreview::OnChangeSetting`).
//!
//! * `prefs`: the eight settings of the application, read from and saved to `[Print]` of the
//!   settings file (`matsu_config::settings::print`).
//! * `model`: Sakura's rules for the values and the numbers the dialog shows. No window.
//! * `dialog`: the dialog.

pub(crate) mod dialog;
pub(crate) mod model;
pub mod prefs;

use gpui::{Context, Window};

use crate::view::EditorView;
pub use prefs::{PrintPrefs, install};

/// The width of the line numbers the dialog assumes when the print preview is not open: the least
/// (`nLineNumberColumns = 3; // ファイルメニューからの設定時は最小値`).
pub(crate) const LINE_NUMBER_COLUMNS_WITHOUT_PREVIEW: u32 = 3;

impl EditorView {
    /// The print setting this window prints with: the one the document type chose among the
    /// eight.
    pub(crate) fn current_print_setting(&self, cx: &gpui::App) -> matsu_print::PrintSetting {
        let type_settings = self.type_settings(cx);
        prefs::current_setting(cx, self.type_index(), &type_settings)
    }

    /// `Command_PRINT_PAGESETUP`: shows the dialog, and applies what OK gives. While the preview
    /// is open the line numbers take the width the document needs; otherwise the least.
    pub(crate) fn print_page_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.print.setup_dialog.is_some() {
            return;
        }
        let type_index = self.type_index();
        let type_settings = self.type_settings(cx);
        let settings = prefs::settings(cx);
        let current = prefs::current_index(cx, type_index, &type_settings);
        let number_columns = self
            .print
            .preview
            .as_ref()
            .map_or(LINE_NUMBER_COLUMNS_WITHOUT_PREVIEW, |preview| {
                preview.read(cx).line_number_columns()
            });
        let (sender, receiver) = async_channel::bounded(1);
        self.open_print_setting_dialog(settings, current, number_columns, sender, window, cx);
        // A macro waits until the dialog is closed and its answer applied.
        let tracked = crate::macros::track(cx);
        cx.spawn_in(window, async move |this, cx| {
            let _tracked = tracked;
            if let Ok(Some(answer)) = receiver.recv().await {
                this.update_in(cx, |this, _, cx| {
                    this.apply_print_setting(&answer, type_index, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// What OK of the dialog does: keeps the settings, and lets every print preview that is open
    /// lay the document out with the setting its window prints with now.
    fn apply_print_setting(
        &mut self,
        answer: &dialog::Answer,
        type_index: usize,
        cx: &mut Context<Self>,
    ) {
        prefs::commit(
            cx,
            &answer.settings,
            &answer.original,
            type_index,
            answer.current,
        );
        self.print_setting_changed(cx);
        let me = cx.entity_id();
        let others: Vec<_> = crate::file_ops::windows::workspaces(cx)
            .into_iter()
            .flat_map(|(_, workspace)| workspace.read(cx).all_views())
            .filter(|view| view.entity_id() != me)
            .collect();
        for view in others {
            view.update(cx, |view, cx| view.print_setting_changed(cx));
        }
    }

    /// `CPrintPreview::OnChangeSetting`: the preview takes the setting of this window again.
    pub(crate) fn print_setting_changed(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = self.print.preview.clone() else {
            return;
        };
        let setting = self.current_print_setting(cx);
        preview.update(cx, |preview, cx| preview.set_setting(setting, cx));
    }
}

#[cfg(test)]
mod tests;
