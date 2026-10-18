//! 「支援」: `IDD_PROP_SUPPORT` (`sakura_core/sakura_rc.rc`), `CPropTypesSupport`
//! (`sakura_core/typeprop/CPropTypesSupport.cpp`).
//!
//! Live: the input completion (入力補完): the word file, 英大文字小文字を同一視, and the sources
//! of the candidates, the words of the document and the highlighted keywords
//! (`crate::support::completion`).
//!
//! Not available yet: the plugin that adds candidates (the list has only なし), the external
//! help files (`F_EXTHELP1`, `F_EXTHTMLHELP`), the warning about mixed line endings on saving
//! (`CCodeChecker::OnCheckSave`), the C/C++ smart indent options (UME Editor has no smart indent)
//! and the detection of the indentation style on loading (`CDocEditor::OnAfterLoad`).

use std::path::{Path, PathBuf};

use super::super::ids::*;
use crate::common_settings::CommonSettingsDialog;
use crate::common_settings::model::{Control, Item};
use crate::file_dialog::{self, Ask};

/// `GetHokanMethodList`: the plugins that complete are added to the list after なし
/// (`STR_SMART_INDENT_NONE`); UME Editor has no plugins that complete.
pub(in crate::type_settings) const HOKAN_TYPES: &[(&str, i32)] = &[("なし", 0)];

/// The path the file dialog gives, as the box takes it: relative to the folder of the settings
/// file when it is under it (`GetRelPath`, `CDlgOpenFile::SelectFile` with `resolvePath`).
pub(in crate::type_settings) fn path_for_box(path: &Path, ini_dir: Option<&Path>) -> String {
    let shown = ini_dir
        .and_then(|dir| path.strip_prefix(dir).ok())
        .unwrap_or(path);
    shown.to_string_lossy().into_owned()
}

/// `IDC_BUTTON_HOKANFILE_REF`: chooses the word file, starting where the box says (a relative
/// path starts at the settings file's folder, `CTextInputStream_AbsIni`).
fn browse_word_file(
    dialog: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    let text = dialog.input_text(HOKAN_FILE, cx).unwrap_or_default();
    let ini_dir: Option<PathBuf> = crate::doc_types::with_user(cx, |user| {
        crate::settings::history_file(user).and_then(|path| path.parent().map(Path::to_owned))
    });
    let typed = Path::new(&text);
    let full = match (&ini_dir, typed.is_relative()) {
        (Some(dir), true) => dir.join(typed),
        _ => typed.to_owned(),
    };
    let start = if text.is_empty() {
        ini_dir.clone()
    } else {
        full.parent().map(Path::to_owned)
    };
    let name = full
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|_| !text.is_empty());
    let receiver = file_dialog::ask(
        window,
        cx,
        Ask::open(false)
            .start(start)
            .name(name)
            .titled("単語ファイルの選択", "選択"),
    );
    cx.spawn_in(window, async move |this, cx| {
        let chosen = receiver.await.ok().flatten();
        if let Some(path) = chosen.and_then(|paths| paths.into_iter().next()) {
            let text = path_for_box(&path, ini_dir.as_deref());
            this.update_in(cx, |this, window, cx| {
                this.set_input(HOKAN_FILE, &text, window, cx)
            })
            .ok();
        }
    })
    .detach();
}

fn no_action(
    _: &mut CommonSettingsDialog,
    _: &mut gpui::Window,
    _: &mut gpui::Context<CommonSettingsDialog>,
) {
}

pub(in crate::type_settings) fn items() -> Vec<Item> {
    vec![
        Item::Group(
            "入力補完機能",
            vec![
                Item::Row(vec![
                    Control::text(HOKAN_FILE, "単語ファイル(&W)", "", "").into(),
                    Control::button(HOKAN_FILE_BROWSE, "(&1)...", browse_word_file).into(),
                    Control::choice(HOKAN_TYPE, "", "", 0, HOKAN_TYPES)
                        .unavailable()
                        .into(),
                ]),
                Item::Row(vec![
                    Control::check(HOKAN_LO_HI_CASE, "英大文字小文字を同一視(&I)", "", false)
                        .into(),
                    Item::Label("候補："),
                    Control::check(HOKAN_BY_FILE, "編集中のファイル(&F)", "", true).into(),
                    Control::check(HOKAN_BY_KEYWORD, "強調キーワード(&K)", "", false).into(),
                ]),
            ],
        ),
        Item::Group(
            "外部ヘルプの設定(&L)",
            vec![Item::Row(vec![
                Control::text(EXT_HELP, "", "", "").unavailable().into(),
                Control::button(EXT_HELP_BROWSE, "(&2)...", no_action)
                    .unavailable()
                    .into(),
            ])],
        ),
        Item::Group(
            "外部HTMLヘルプの設定(&P)",
            vec![
                Item::Row(vec![
                    Control::text(EXT_HTML_HELP, "", "", "")
                        .unavailable()
                        .into(),
                    Control::button(EXT_HTML_HELP_BROWSE, "(&3)...", no_action)
                        .unavailable()
                        .into(),
                ]),
                Control::check(HTML_HELP_SINGLE, "ビューアを複数起動しない(&N)", "", true)
                    .unavailable()
                    .into(),
            ],
        ),
        Control::check(
            CHK_ENTER_AT_END,
            "保存時に改行コードの混在を警告する(&E)",
            "",
            true,
        )
        .unavailable()
        .into(),
        Item::Group(
            "C/C++インデント詳細設定",
            vec![
                Item::Row(vec![
                    Control::check(INDENT_CPP_STRING, "文字列を無視する(&S)", "", true)
                        .unavailable()
                        .into(),
                    Control::check(INDENT_CPP_COMMENT, "コメントを無視する(&C)", "", true)
                        .unavailable()
                        .into(),
                ]),
                Control::check(INDENT_CPP_UNDO, "Undoバッファを分ける(&U)", "", false)
                    .unavailable()
                    .into(),
            ],
        ),
        Control::check(
            DETECT_INDENT,
            "ファイル読み込み時にインデントスタイルを検出する(&T)",
            "",
            true,
        )
        .unavailable()
        .into(),
    ]
}
