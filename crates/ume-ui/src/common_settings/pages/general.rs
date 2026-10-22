//! 「全般」: `IDD_PROP_GENERAL` (`sakura_core/sakura_rc.rc`), `CPropGeneral`
//! (`sakura_core/prop/CPropComGeneral.cpp`).
//!
//! Live: the two "stop at both ends" options of the cursor, and the number of files and folders
//! in the history (and the buttons that clear them). Not available yet: the free cursor, the
//! mouse activation, the caret shape, the task tray (UME Editor is one process, ADR-0016), the
//! confirmations when several windows close, the repeated scroll and the wheel combinations
//! (the wheel is the key table's, `crate::view::paging`), and the screen cache.

use gpui::{Context, Window};
use ume_commands::edit_options;

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::model::{Control, Item, Values};

/// `SpecialScrollModeArr` (`CPropComGeneral.cpp`): the combinations of the wheel, with the
/// numbers Sakura writes (`MOUSEFUNCTION_*`, `VK_CONTROL`, `VK_SHIFT`).
const WHEEL_COMBINATIONS: &[(&str, i32)] = &[
    ("組み合わせなし", 0),
    ("マウス中ボタン", 2),
    ("マウスサイドボタン1", 3),
    ("マウスサイドボタン2", 4),
    ("CONTROLキー", 17),
    ("SHIFTキー", 16),
];

/// The most files and folders the history keeps (`MAX_MRU`, `MAX_OPENFOLDER`).
const MAX_HISTORY: i32 = ume_config::history::MAX_MRU as i32;

fn tray_is_used(values: &Values) -> bool {
    values.bool("use_tray_icon")
}

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![Item::Columns(vec![
        vec![
            Item::Group(
                "カーソル",
                vec![
                    Control::check(
                        "free_caret",
                        "フリーカーソル(&F)",
                        "bIsFreeCursorMode",
                        false,
                    )
                    .unavailable()
                    .into(),
                    Control::check(
                        "stops_both_ends_word",
                        "単語単位で移動するときに\n単語の両端に止まる(&B)",
                        "bStopsBothEndsWhenSearchWord",
                        false,
                    )
                    .into(),
                    Control::check(
                        "stops_both_ends_paragraph",
                        "段落単位で移動するときに\n段落の両端に止まる(&P)",
                        "bStopsBothEndsWhenSearchParagraph",
                        false,
                    )
                    .into(),
                    Control::check(
                        "no_move_activate_by_mouse",
                        "マウスクリックでのアクティブ化では\nカーソルを移動しない(&O)",
                        "NoCaretMoveByActivation",
                        false,
                    )
                    .unavailable()
                    .into(),
                ],
            ),
            Item::Group(
                "カーソル形状",
                vec![Item::Row(vec![
                    Control::radio("caret_type", "&Windows風", "nCaretType", 0, 0)
                        .unavailable()
                        .into(),
                    Control::radio("caret_type", "MS-&DOS風", "nCaretType", 1, 0)
                        .unavailable()
                        .into(),
                ])],
            ),
            Item::Group(
                "タスクトレイ",
                vec![
                    Control::check(
                        "use_tray_icon",
                        "タスクトレイを使う(&T)",
                        "bTaskTrayUse",
                        true,
                    )
                    .unavailable()
                    .into(),
                    Control::check(
                        "stay_task_tray",
                        "タスクトレイに常駐(&R)",
                        "bTaskTrayStay",
                        false,
                    )
                    .unavailable()
                    .when(tray_is_used)
                    .into(),
                    Control::text(
                        "tray_menu_hot_key",
                        "左クリックメニューのショートカットキー(&K)",
                        "",
                        "Ctrl+Alt+Z",
                    )
                    .unavailable()
                    .into(),
                    Control::check(
                        "close_all_confirm",
                        "同時に複数の編集用ウィンドウを\n閉じるとき確認(&U)",
                        "bCloseAllConfirm",
                        false,
                    )
                    .unavailable()
                    .into(),
                    Control::check(
                        "exit_confirm",
                        "ウメエディタの全終了で編集用\nウィンドウを閉じるとき確認(&V)",
                        "bExitConfirm",
                        false,
                    )
                    .unavailable()
                    .into(),
                ],
            ),
        ],
        vec![
            Item::Group(
                "スクロール",
                vec![
                    Control::number(
                        "repeated_scroll_lines",
                        "行数(&N)",
                        "nRepeatedScrollLineNum",
                        3,
                        1,
                        10,
                    )
                    .unavailable()
                    .into(),
                    Control::check(
                        "repeated_scroll_smooth",
                        "少し滑らかにする(&S)",
                        "nRepeatedScroll_Smooth",
                        false,
                    )
                    .unavailable()
                    .into(),
                    Control::choice(
                        "wheel_page_scroll",
                        "組み合わせてホイール操作した時\nページスクロールする(&J)",
                        "nPageScrollByWheel",
                        0,
                        WHEEL_COMBINATIONS,
                    )
                    .unavailable()
                    .into(),
                    Control::choice(
                        "wheel_hscroll",
                        "組み合わせてホイール操作した時\n横スクロールする(&H)",
                        "nHorizontalScrollByWheel",
                        0,
                        WHEEL_COMBINATIONS,
                    )
                    .unavailable()
                    .into(),
                    Control::check(
                        "mem_dc",
                        "画面キャッシュを使う(&G)",
                        "bUseCompotibleBMP",
                        true,
                    )
                    .unavailable()
                    .into(),
                ],
            ),
            Item::Group(
                "履歴",
                vec![
                    Control::number(
                        "max_mru_file",
                        "ファイルの履歴&MAX",
                        "nMRUArrNum_MAX",
                        15,
                        0,
                        MAX_HISTORY,
                    )
                    .into(),
                    Control::button("clear_mru_file", "履歴をクリア(&C)...", clear_files).into(),
                    Control::number(
                        "max_mru_folder",
                        "フォルダーの履歴MA&X",
                        "nOPENFOLDERArrNum_MAX",
                        15,
                        0,
                        MAX_HISTORY,
                    )
                    .into(),
                    Control::button("clear_mru_folder", "履歴をクリア(&L)...", clear_folders)
                        .into(),
                ],
            ),
        ],
    ])]
}

/// `IDC_BUTTON_CLEAR_MRU_FILE`: asks, clears the history of files, says so (`STR_PROPCOMGEN_FILE1`,
/// `STR_PROPCOMGEN_FILE2`). It acts at once, not when OK is pressed.
fn clear_files(
    dialog: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    dialog.confirm_then(
        "最近使ったファイルの履歴を削除します。\nよろしいですか？\n",
        "最近使ったファイルの履歴を削除しました。\n",
        window,
        cx,
        |cx| {
            crate::history::update(cx, |h| h.files.clear());
        },
    );
}

/// `IDC_BUTTON_CLEAR_MRU_FOLDER` (`STR_PROPCOMGEN_DIR1`, `STR_PROPCOMGEN_DIR2`).
fn clear_folders(
    dialog: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    dialog.confirm_then(
        "最近使ったフォルダーの履歴を削除します。\nよろしいですか？\n",
        "最近使ったフォルダーの履歴を削除しました。\n",
        window,
        cx,
        |cx| {
            crate::history::update(cx, |h| h.folders.clear());
        },
    );
}

/// What OK does to the windows: the options of the commands, and the lists of the menus (the
/// history reads its size from the file).
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, before: Option<&Values>) {
    let mut options = edit_options::get();
    options.stops_both_ends_word = after.bool("stops_both_ends_word");
    options.stops_both_ends_paragraph = after.bool("stops_both_ends_paragraph");
    edit_options::set(options);
    let history_changed = before.is_none_or(|b| {
        b.int("max_mru_file") != after.int("max_mru_file")
            || b.int("max_mru_folder") != after.int("max_mru_folder")
    });
    if history_changed {
        crate::app::refresh_menus(env.cx);
    }
}
