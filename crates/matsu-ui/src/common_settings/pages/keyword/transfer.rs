//! インポート and エクスポート: the keyword definition file (`.kwd`) of the chosen set
//! (`CPropKeyword::Import_List_KeyWord`, `Export_List_KeyWord`, `CImpExpKeyWord`).

use std::path::Path;

use gpui::{Context, Window};
use matsu_config::settings::keyword_file;

use super::{CANNOT_OPEN, EXPORTED, IMPORT_FULL, current, entry, say};
use crate::common_settings::dialog::CommonSettingsDialog;
use crate::file_dialog::Ask;

/// `GetDefaultExtension` of `CImpExpKeyWord`.
const WILDCARD: &str = "*.kwd";

impl CommonSettingsDialog {
    /// インポート: asks for a file and adds its keywords to the chosen set.
    pub(super) fn keyword_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pick_settings_file(
            Ask::open(false).titled("キーワードのインポート", "選択"),
            WILDCARD,
            window,
            cx,
            |this, path, window, cx| {
                this.keyword_import_from(path, window, cx);
            },
        );
    }

    /// Adds the keywords of the file `path` to the chosen set, and takes the case rule of the file
    /// (`CImpExpKeyWord::Import`). Returns whether the file was read.
    pub(super) fn keyword_import_from(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(at) = current(self.values()) else {
            return false;
        };
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(_) => {
                say(window, cx, &format!("{CANNOT_OPEN}{}", path.display()));
                return false;
            }
        };
        let mut entries = self.keyword_entries();
        let out_of_room = entry::import(&mut entries, at, &keyword_file::parse(&bytes));
        self.keyword_put(&entries, cx);
        // The keywords were added: none is chosen.
        self.keyword_select_word(None, window, cx);
        if out_of_room {
            say(window, cx, IMPORT_FULL);
        }
        true
    }

    /// エクスポート: asks for a file name (the name of the set with `.kwd`) and writes the chosen set.
    pub(super) fn keyword_export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = current(self.values()) else {
            return;
        };
        let name = self
            .values()
            .rows(super::SETS)
            .get(at)
            .and_then(|row| row.get(1))
            .map(|name| keyword_file::export_file_name(name));
        self.pick_settings_file(
            Ask::save()
                .name(name)
                .titled("キーワードのエクスポート", "保存"),
            WILDCARD,
            window,
            cx,
            |this, path, window, cx| {
                this.keyword_export_to(path, window, cx);
            },
        );
    }

    /// Writes the chosen set to the file `path` (`CImpExpKeyWord::Export`). Returns whether it was written.
    pub(super) fn keyword_export_to(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(at) = current(self.values()) else {
            return false;
        };
        let set = &self.keyword_entries()[at];
        let bytes = keyword_file::render(&set.name, set.case_sensitive, &set.words);
        if std::fs::write(path, bytes).is_err() {
            say(window, cx, &format!("{CANNOT_OPEN}{}", path.display()));
            return false;
        }
        say(window, cx, &format!("{EXPORTED}{}", path.display()));
        true
    }
}
