//! インポート and エクスポート of the colors of the type: `CPropTypesColor::Import` and `Export`,
//! `CImpExpColors` (`sakura_core/typeprop/CPropTypesColor.cpp`, `CImpExpManager.cpp`), the `.col` file.
//!
//! The file is a settings file: the first line `; テキストエディタ色設定 Ver3` and the section
//! `[SakuraColor]` with a key `C[NAME]` for each color (`matsu_config::settings::type_file`).
//! Importing puts the colors of the file into the page, over the colors it has now (a color the
//! file lacks stays); nothing is written to the settings file until OK. Exporting writes the colors
//! the page shows now. Sakura offers no file name for the export (`SetBaseName` is not called).

use std::path::Path;

use gpui::{Context, Window};
use matsu_config::settings::ColorSet;
use matsu_config::settings::type_file::{COLOR_WILDCARD, parse_colors, render_colors};

use super::super::ids::*;
use super::super::values::{color_of, color_value};
use super::color::sync;
use crate::common_settings::CommonSettingsDialog;
use crate::common_settings::model::{Value, Values};
use crate::common_settings::transfer::{cannot_export, cannot_open, exported, say};
use crate::file_dialog::Ask;

/// `STR_IMPEXP_ERR_COLOR_OLD`
const OLD_FORMAT: &str = "色設定ファイルの形式が違います。古い形式はサポートされなくなりました。\n";

/// The colors the page holds now.
pub(in crate::type_settings) fn colors_of(values: &Values) -> ColorSet {
    let mut colors = ColorSet::default();
    for (item, name) in ColorSet::all_names().enumerate() {
        if let Some(entry) = colors.get_mut(name) {
            entry.disp = values.bool(item_key(item, Field::Disp));
            entry.bold = values.bool(item_key(item, Field::Bold));
            entry.underline = values.bool(item_key(item, Field::Underline));
            entry.text = color_of(values.int(item_key(item, Field::Text)));
            entry.back = color_of(values.int(item_key(item, Field::Back)));
        }
    }
    colors
}

/// Puts `colors` into the page, and shows the chosen item again (`SetData`).
pub(in crate::type_settings) fn put_colors(values: &mut Values, colors: &ColorSet) {
    for (item, name) in ColorSet::all_names().enumerate() {
        let Some(entry) = colors.get(name) else {
            continue;
        };
        values.set(item_key(item, Field::Disp), Value::Bool(entry.disp));
        values.set(item_key(item, Field::Bold), Value::Bool(entry.bold));
        values.set(
            item_key(item, Field::Underline),
            Value::Bool(entry.underline),
        );
        values.set(
            item_key(item, Field::Text),
            Value::Int(color_value(entry.text)),
        );
        values.set(
            item_key(item, Field::Back),
            Value::Int(color_value(entry.back)),
        );
    }
    sync(values, COLOR_SEL);
}

impl CommonSettingsDialog {
    /// 「インポート」: asks for a file and takes its colors.
    pub(in crate::type_settings) fn colors_import(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pick_settings_file(
            Ask::open(false).titled("色設定のインポート", "インポート"),
            COLOR_WILDCARD,
            window,
            cx,
            |this, path, window, cx| {
                this.colors_import_from(path, window, cx);
            },
        );
    }

    /// `CImpExpColors::Import` of `path`. Returns whether the colors were taken.
    pub(in crate::type_settings) fn colors_import_from(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Ok(bytes) = std::fs::read(path) else {
            say(&cannot_open(path), window, cx);
            return false;
        };
        let Some(colors) = parse_colors(&bytes, &colors_of(self.values())) else {
            say(&format!("{OLD_FORMAT}{}", path.display()), window, cx);
            return false;
        };
        self.edit_values(|values| put_colors(values, &colors), cx);
        true
    }

    /// 「エクスポート」: asks for a name and writes the colors.
    pub(in crate::type_settings) fn colors_export(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pick_settings_file(
            Ask::save().titled("色設定のエクスポート", "保存"),
            COLOR_WILDCARD,
            window,
            cx,
            |this, path, window, cx| {
                this.colors_export_to(path, window, cx);
            },
        );
    }

    /// `CImpExpColors::Export` to `path`. Returns whether the file was written.
    pub(in crate::type_settings) fn colors_export_to(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let written = std::fs::write(path, render_colors(&colors_of(self.values()))).is_ok();
        say(
            &if written {
                exported(path)
            } else {
                cannot_export(path)
            },
            window,
            cx,
        );
        written
    }
}
