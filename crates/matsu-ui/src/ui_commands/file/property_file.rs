//! `F_PROPERTY_FILE`: shows the properties of the file.
//!
//! Sakura: `Command_PROPERTY_FILE` opens `CDlgProperty` (`sakura_core/dlg/CDlgProperty.cpp`), a
//! read-only box of text. The text is built in `file_ops::property` and shown in a message box
//! with an OK button.

use crate::file_ops::ask::tell;
use crate::file_ops::property::{PropertyInfo, disk_info, property_text};
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, PromptLevel, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct PropertyFile;

impl UiCommand for PropertyFile {
    const ID: &'static str = "property_file";
    const SAKURA: &'static str = "F_PROPERTY_FILE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let text = property_text(&view.property_info(cx));
        start(window, cx, async move |_view, cx| {
            tell(cx, PromptLevel::Info, "ファイルのプロパティ", Some(&text)).await;
        });
        Ok(())
    }
}

register_ui_command!(PropertyFile);

impl EditorView {
    /// What the property dialog shows about the document of this window.
    fn property_info(&self, cx: &gpui::App) -> PropertyInfo {
        let shared = self.document().read(cx);
        let file = shared.file();
        PropertyInfo {
            path: shared.path().map(|p| p.display().to_string()),
            encoding: file.encoding.combo_name().to_owned(),
            bom: file.bom,
            line_ending: crate::status::eol_name(shared.new_line()).to_owned(),
            lines: shared.doc().line_count(),
            rows: self
                .layout()
                .is_complete()
                .then(|| self.layout().row_count()),
            view_mode: shared.view_mode(),
            modified: shared.model().is_modified(),
            lock: shared.held_lock_mode(),
            disk: shared.path().and_then(disk_info),
        }
    }
}
