//! 「全般」: `IDD_PROP_GENERAL` (`sakura_core/sakura_rc.rc`), `CPropGeneral`
//! (`sakura_core/prop/CPropComGeneral.cpp`).
//!
//! Live: the two "stop at both ends" options of the cursor, the number of files and folders in the
//! history (and the buttons that clear them), the two confirmations before windows close
//! ([`CloseConfirm`]), and the scrolling ([`ScrollOptions`]): the rows a held up or down key
//! moves, and the keys and buttons that turn the wheel into a page or a sideways scroll
//! (`crate::view::paging`), the free cursor (`matsu_commands::context`'s `free_cursor`) and the
//! shape of the caret (`crate::view::caret_shape`). Not available yet: the mouse activation, the
//! task tray (MATSU Editor is one process, ADR-0016), "少し滑らかにする" (it only
//! redraws between the steps, which GPUI cannot do inside a command; the result of the move is
//! the same) and the screen cache (GPUI draws the window itself; there is no device context to
//! keep).

use gpui::{App, Context, Global, Window};
use matsu_commands::edit_options;

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::model::{Control, Item, Values};

/// `SpecialScrollModeArr` (`CPropComGeneral.cpp`): the combinations of the wheel, with the
/// numbers Sakura writes (`MOUSEFUNCTION_*`, `VK_CONTROL`, `VK_SHIFT`).
const WHEEL_COMBINATIONS: &[(&str, i32)] = &[
    ("組み合わせなし", 0),
    ("マウス中ボタン", WHEEL_MIDDLE_BUTTON),
    ("マウスサイドボタン1", WHEEL_SIDE_BUTTON_1),
    ("マウスサイドボタン2", WHEEL_SIDE_BUTTON_2),
    ("CONTROLキー", WHEEL_CONTROL_KEY),
    ("SHIFTキー", WHEEL_SHIFT_KEY),
];

/// The most files and folders the history keeps (`MAX_MRU`, `MAX_OPENFOLDER`).
const MAX_HISTORY: i32 = matsu_config::history::MAX_MRU as i32;

/// The questions before several windows are closed: `m_bCloseAllConfirm` and `m_bExitConfirm`
/// (both off by default, `CShareData::InitShareData`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CloseConfirm {
    /// Closing several windows at once asks first (`CAppNodeGroupHandle::RequestCloseEditor`,
    /// `STR_ERR_CSHAREDATA19`).
    pub close_all: bool,
    /// 全終了 asks first (`CControlTray::TerminateApplication`, `STR_TRAY_EXITALL`).
    pub exit: bool,
}

impl Global for CloseConfirm {}

/// The questions now.
pub(crate) fn close_confirm(cx: &App) -> CloseConfirm {
    cx.try_global::<CloseConfirm>().copied().unwrap_or_default()
}

/// The scrolling options (`CommonSetting::m_sGeneral`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScrollOptions {
    /// `m_nRepeatedScrollLineNum`: the rows that a held up or down key moves (3 by default).
    pub repeated_lines: usize,
    /// `m_nPageScrollByWheel`: the key or button that makes the wheel scroll by pages
    /// ([`WHEEL_COMBINATIONS`]; 0 for none).
    pub page_by_wheel: i32,
    /// `m_nHorizontalScrollByWheel`: the same for scrolling sideways.
    pub hscroll_by_wheel: i32,
}

impl Default for ScrollOptions {
    fn default() -> Self {
        Self {
            repeated_lines: 3,
            page_by_wheel: 0,
            hscroll_by_wheel: 0,
        }
    }
}

impl Global for ScrollOptions {}

/// The scrolling options now.
pub(crate) fn scroll_options(cx: &App) -> ScrollOptions {
    cx.try_global::<ScrollOptions>()
        .copied()
        .unwrap_or_default()
}

/// `MOUSEFUNCTION_CENTER`, `MOUSEFUNCTION_LEFTSIDE`, `MOUSEFUNCTION_RIGHTSIDE` and the virtual
/// keys of `WHEEL_COMBINATIONS` (`CEditView::IsSpecialScrollMode`).
pub(crate) const WHEEL_MIDDLE_BUTTON: i32 = 2;
pub(crate) const WHEEL_SIDE_BUTTON_1: i32 = 3;
pub(crate) const WHEEL_SIDE_BUTTON_2: i32 = 4;
pub(crate) const WHEEL_SHIFT_KEY: i32 = 16;
pub(crate) const WHEEL_CONTROL_KEY: i32 = 17;

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
                    Control::radio("caret_type", "&Windows風", "nCaretType", 0, 0).into(),
                    Control::radio("caret_type", "MS-&DOS風", "nCaretType", 1, 0).into(),
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
                    .into(),
                    Control::check(
                        "exit_confirm",
                        "マツエディタの全終了で編集用\nウィンドウを閉じるとき確認(&V)",
                        "bExitConfirm",
                        false,
                    )
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
                    .into(),
                    Control::choice(
                        "wheel_hscroll",
                        "組み合わせてホイール操作した時\n横スクロールする(&H)",
                        "nHorizontalScrollByWheel",
                        0,
                        WHEEL_COMBINATIONS,
                    )
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

/// The options of the commands that this page sets, on top of `options`.
pub(in crate::common_settings) fn general_options(
    mut options: edit_options::EditOptions,
    after: &Values,
) -> edit_options::EditOptions {
    options.stops_both_ends_word = after.bool("stops_both_ends_word");
    options.stops_both_ends_paragraph = after.bool("stops_both_ends_paragraph");
    options.free_cursor = after.bool("free_caret");
    // `GetCaretType()`: the low 4 bits of `nCaretType`; 1 is the MS-DOS style, anything else the
    // Windows style (`ShowEditCaret` tests for 0 and 1 only).
    options.dos_caret = after.int("caret_type") & 0xf == 1;
    options
}

/// What OK does to the windows: the options of the commands, and the lists of the menus (the
/// history reads its size from the file).
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, before: Option<&Values>) {
    edit_options::set(general_options(edit_options::get(), after));
    env.cx.set_global(ScrollOptions {
        repeated_lines: after.int("repeated_scroll_lines").clamp(1, 10) as usize,
        page_by_wheel: after.int("wheel_page_scroll"),
        hscroll_by_wheel: after.int("wheel_hscroll"),
    });
    env.cx.set_global(CloseConfirm {
        close_all: after.bool("close_all_confirm"),
        exit: after.bool("exit_confirm"),
    });
    let history_changed = before.is_none_or(|b| {
        b.int("max_mru_file") != after.int("max_mru_file")
            || b.int("max_mru_folder") != after.int("max_mru_folder")
    });
    if history_changed {
        crate::app::refresh_menus(env.cx);
    }
}
