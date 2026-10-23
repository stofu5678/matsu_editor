//! 「編集」: `IDD_PROP_EDIT` (`sakura_core/sakura_rc.rc`), `CPropEdit`
//! (`sakura_core/prop/CPropComEdit.cpp`).
//!
//! Live: copy without a selection, paste of a row at the start of the row, the conversion of line
//! endings in a paste, the copy of wrapped rows with a line ending, the deselection after a copy,
//! the options of the overwrite mode, drag and drop inside the window, selecting a URL by a click,
//! and where the file dialogs start (`eOpenDialogDir`, `szOpenDialogSelDir`, read by
//! `crate::file_dialog`). Not available yet: the lock of the box selection, and the Vista style
//! dialog: MATSU asks GPUI for the dialogs, which on Windows are always the Vista style (the
//! `IFileOpenDialog` in gpui-pre-windows 0.3.8, `platform.rs`), so the old style that the box
//! would switch to does not exist.

use gpui::{Context, Window};
use matsu_commands::edit_options;

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::model::{Control, Item, Values};

/// `OPENDIALOGDIR_CUR`, `OPENDIALOGDIR_MRU`, `OPENDIALOGDIR_SEL` (`eOpenDialogDir`).
const DIR_CURRENT: i32 = 0;
const DIR_RECENT: i32 = 1;
const DIR_SELECTED: i32 = 2;

fn drag_drop_is_on(values: &Values) -> bool {
    values.bool("use_ole_drag_drop")
}

/// `EnableEditPropInput`: the given folder is edited only when it is the one that is chosen.
fn given_folder_is_chosen(values: &Values) -> bool {
    values.int("open_dialog_dir") == DIR_SELECTED
}

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![Item::Columns(vec![
        vec![
            Item::Group(
                "コピー",
                vec![
                    Control::check(
                        "add_crlf_when_copy",
                        "折り返し行に改行を付けてコピー(&C)",
                        "bAddCRLFWhenCopy",
                        false,
                    )
                    .into(),
                    Control::check(
                        "copy_and_disable_selection",
                        "コピーしたら選択解除(&R)",
                        "bCopyAndDisablSelection",
                        false,
                    )
                    .into(),
                    Control::check(
                        "enable_no_select_copy",
                        "選択なしでコピーを可能にする(&E)",
                        "bEnableNoSelectCopy",
                        true,
                    )
                    .into(),
                    Control::check(
                        "enable_line_mode_paste",
                        "ラインモード貼り付けを可能にする(&L)",
                        "bEnableLineModePaste",
                        true,
                    )
                    .into(),
                    Control::check(
                        "convert_eol_paste",
                        "改行コードを変換して貼り付ける(&P)",
                        "bConvertEOLPaste",
                        false,
                    )
                    .into(),
                ],
            ),
            Item::Group(
                "ドラッグ && ドロップ",
                vec![
                    Control::check(
                        "use_ole_drag_drop",
                        "ドラッグ && ドロップ編集する(&D)",
                        "bUseOLE_DragDrop",
                        true,
                    )
                    .into(),
                    Control::check(
                        "use_ole_drop_source",
                        "ドロップ元にする(&S)",
                        "bUseOLE_DropSource",
                        true,
                    )
                    .when(drag_drop_is_on)
                    .into(),
                ],
            ),
            Item::Group(
                "上書きモード",
                vec![
                    Control::check(
                        "not_overwrite_crlf",
                        "改行コードは上書きしない(&N)",
                        "bNotOverWriteCRLF",
                        true,
                    )
                    .into(),
                    Control::check(
                        "overwrite_fix_mode",
                        "文字幅に合わせてスペースを詰める(&F)",
                        "bOverWriteFixMode",
                        false,
                    )
                    .into(),
                    Control::check(
                        "overwrite_box_delete",
                        "矩形入力で選択範囲を削除する(&G)",
                        "bOverWriteBoxDelete",
                        false,
                    )
                    .into(),
                ],
            ),
        ],
        vec![
            Item::Group(
                "クリッカブルURL",
                vec![
                    Control::check(
                        "select_clicked_url",
                        "クリックで&URLを選択する",
                        "bSelectClickedURL",
                        true,
                    )
                    .into(),
                ],
            ),
            Item::Group(
                "ファイルダイアログの初期位置",
                vec![
                    Control::radio(
                        "open_dialog_dir",
                        "カレントフォルダー(&U)",
                        "eOpenDialogDir",
                        DIR_CURRENT,
                        DIR_CURRENT,
                    )
                    .into(),
                    Control::radio(
                        "open_dialog_dir",
                        "最近使ったフォルダー(&M)",
                        "eOpenDialogDir",
                        DIR_RECENT,
                        DIR_CURRENT,
                    )
                    .into(),
                    Control::radio(
                        "open_dialog_dir",
                        "指定フォルダー(&F)",
                        "eOpenDialogDir",
                        DIR_SELECTED,
                        DIR_CURRENT,
                    )
                    .into(),
                    Item::Row(vec![
                        Control::text(
                            "open_dialog_sel_dir",
                            "",
                            "szOpenDialogSelDir",
                            matsu_config::dialog_dir::DEFAULT_SELECTED,
                        )
                        .named("指定フォルダー")
                        .when(given_folder_is_chosen)
                        .into(),
                        Control::button("open_dialog_sel_dir_browse", "...", browse)
                            .named("指定フォルダーを参照")
                            .when(given_folder_is_chosen)
                            .into(),
                    ]),
                ],
            ),
            Control::check(
                "vista_style_file_dialog",
                "Vistaスタイルのファイルダイアログ(&I)",
                "bVistaStyleFileDialog",
                true,
            )
            .unavailable()
            .into(),
            Item::Group(
                "その他",
                vec![
                    Control::check(
                        "enable_ext_eol",
                        "改行コードNEL, PS, LSを有効にする(&O)",
                        "bEnableExtEol",
                        false,
                    )
                    .into(),
                    Control::check(
                        "box_select_lock",
                        "矩形選択移動で選択をロックする(&B)",
                        "bBoxSelectLock",
                        true,
                    )
                    .unavailable()
                    .into(),
                ],
            ),
        ],
    ])]
}

/// `IDC_BUTTON_FILEOPENDIR`: chooses the given folder. The box starts the dialog (its meta
/// folders expanded) and takes the folder back with `%` doubled (`STR_PROPEDIT_SELECT_DIR`).
fn browse(
    dialog: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    dialog.choose_folder(
        "open_dialog_sel_dir",
        "ファイルダイアログの指定フォルダーの選択",
        window,
        cx,
    );
}

/// What OK does to the windows: the options of the commands. The start folder of the dialogs is
/// read from the settings each time a dialog opens.
pub(in crate::common_settings) fn apply(_env: &mut Env, after: &Values, _before: Option<&Values>) {
    let mut options = edit_options::get();
    options.enable_no_select_copy = after.bool("enable_no_select_copy");
    options.enable_line_mode_paste = after.bool("enable_line_mode_paste");
    options.convert_eol_paste = after.bool("convert_eol_paste");
    // Open documents read it on their next edit (`SharedDocument::parts_mut`), as Sakura reads it
    // where it makes a line; the lines that exist are not split again.
    options.ext_eol = after.bool("enable_ext_eol");
    // The mouse reads them at each press (`view/mouse.rs`).
    options.drag_drop = after.bool("use_ole_drag_drop");
    options.drop_source = after.bool("use_ole_drop_source");
    options.select_clicked_url = after.bool("select_clicked_url");
    options.add_crlf_when_copy = after.bool("add_crlf_when_copy");
    options.copy_and_disable_selection = after.bool("copy_and_disable_selection");
    options.not_overwrite_crlf = after.bool("not_overwrite_crlf");
    options.overwrite_fix_mode = after.bool("overwrite_fix_mode");
    options.overwrite_box_delete = after.bool("overwrite_box_delete");
    edit_options::set(options);
}
