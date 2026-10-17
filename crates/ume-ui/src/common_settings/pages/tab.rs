//! 「タブバー」: `IDD_PROP_TAB` (`sakura_core/sakura_rc.rc`), `CPropTab`
//! (`sakura_core/prop/CPropComTab.cpp`).
//!
//! Live: the bar (`bDispTabWnd`), the grouping (`bDispTabWndMultiWin`, the opposite), the new
//! window for a file from outside (`bNewWindow`), what happens when the last file closes and when
//! the close button of the window is used (`bTab_RetainEmptyWin`, `bTab_CloseOneWin`), the width of
//! the tabs (`bSameTabWidth`), their close buttons (`bDispTabClose`) and the wheel over the bar
//! (`bChgWndByWheel`). Not available yet: the icons, the font, the sorted list, the rows, the
//! place and the caption of the tabs (the caption needs `CFormatManager::ExpandParameter`).
//!
//! `EnableTabPropInput` (`CPropComTab.cpp`) says which controls can be used: all of them need the
//! bar; the last three of the mode need the grouping too.

use gpui::AnyWindowHandle;

use super::super::Env;
use super::super::model::{Control, Item, Values};
use crate::tabs::flows;
use crate::tabs::settings::{self, DispTabClose, TabOptions};

/// `DispTabCloseArr` (`CPropComTab.cpp`).
const DISP_CLOSE: &[(&str, i32)] = &[("なし", 0), ("常に表示", 1), ("自動表示", 2)];
/// `TabPosArr`: the tab place (`TabPosition_Top`, `TabPosition_Bottom`).
const POSITION: &[(&str, i32)] = &[("上", 0), ("下", 1)];

/// Sakura's default format of the tab caption (`CShareData::InitShareData`).
const CAPTION: &str = "${w?【Grep】$h$:【アウトプット】$:$f$n$}${U?(更新)$}${R?(ビューモード)$:(上書き禁止)$}${M?【キーマクロの記録中】$}";

fn bar_is_shown(values: &Values) -> bool {
    values.bool("tab_bar")
}

fn bar_is_grouped(values: &Values) -> bool {
    values.bool("tab_bar") && values.bool("tab_grouped")
}

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![
        Control::check("tab_bar", "タブバーを表示する(&D)", "bDispTabWnd", false).into(),
        Item::Group(
            "動作モード",
            vec![
                Control::check(
                    "tab_grouped",
                    "ウィンドウをまとめてグループ化する(&B)",
                    "bDispTabWndMultiWin",
                    true,
                )
                .inverted()
                .when(bar_is_shown)
                .into(),
                Control::check(
                    "tab_retain_empty",
                    "最後のファイルを閉じたとき(無題)文書を残す(&R)",
                    "bTab_RetainEmptyWin",
                    true,
                )
                .when(bar_is_grouped)
                .into(),
                Control::check(
                    "tab_close_one_win",
                    "ウィンドウの閉じるボタンは現在のファイルのみ閉じる(&C)",
                    "bTab_CloseOneWin",
                    false,
                )
                .when(bar_is_grouped)
                .into(),
                Control::check(
                    "tab_open_new_win",
                    "外部から起動するときは新しいウインドウで開く(&O)",
                    "bNewWindow",
                    false,
                )
                .when(bar_is_grouped)
                .into(),
            ],
        ),
        Item::Group(
            "タブの外観",
            vec![
                Item::Row(vec![
                    Control::check("tab_icon", "アイコン表示(&I)", "bDispTabIcon", false)
                        .unavailable()
                        .when(bar_is_shown)
                        .into(),
                    Control::check("tab_same_width", "等幅(&E)", "bSameTabWidth", false)
                        .when(bar_is_shown)
                        .into(),
                    Control::choice(
                        "tab_disp_close",
                        "閉じるボタン(&X)",
                        "bDispTabClose",
                        0,
                        DISP_CLOSE,
                    )
                    .when(bar_is_shown)
                    .into(),
                    Control::button("tab_font", "フォント(&F)...", |_, _, _| {})
                        .unavailable()
                        .when(bar_is_shown)
                        .into(),
                ]),
                Item::Row(vec![
                    Control::check(
                        "tab_sort_list",
                        "タブ一覧をソートする(&S)",
                        "bSortTabList",
                        true,
                    )
                    .unavailable()
                    .when(bar_is_shown)
                    .into(),
                    Control::check("tab_multi_line", "多段(&M)", "bTabMultiLine", false)
                        .unavailable()
                        .when(bar_is_shown)
                        .into(),
                    Control::choice("tab_position", "表示位置(&P)", "eTabPosition", 0, POSITION)
                        .unavailable()
                        .when(bar_is_shown)
                        .into(),
                ]),
                Control::text("tab_caption", "タイトル(&T)", "szTabWndCaption", CAPTION)
                    .unavailable()
                    .when(bar_is_shown)
                    .into(),
            ],
        ),
        Item::Group(
            "その他",
            vec![
                Control::check(
                    "tab_change_by_wheel",
                    "マウスホイールでタブ切替(&W)",
                    "bChgWndByWheel",
                    false,
                )
                .when(bar_is_shown)
                .into(),
            ],
        ),
    ]
}

/// The options of the tabs, from the values.
fn options(values: &Values) -> TabOptions {
    TabOptions {
        retain_empty: values.bool("tab_retain_empty"),
        close_one_win: values.bool("tab_close_one_win"),
        same_width: values.bool("tab_same_width"),
        disp_close: DispTabClose::from_code(values.int("tab_disp_close")),
        change_by_wheel: values.bool("tab_change_by_wheel"),
    }
}

/// Why OK cannot go on: a tab that is being read cannot move to another window, so the bar and
/// the grouping cannot change meanwhile (the same refusal as `F_SHOWTAB`).
pub(in crate::common_settings) fn blocked(
    cx: &mut gpui::App,
    after: &Values,
    before: &Values,
) -> Option<&'static str> {
    let changes = after.bool("tab_bar") != before.bool("tab_bar")
        || after.bool("tab_grouped") != before.bool("tab_grouped");
    let reading = changes
        && crate::file_ops::windows::workspaces(cx)
            .into_iter()
            .any(|(handle, _)| crate::file_ops::windows::has_pending(cx, handle));
    reading.then_some("ファイルの読み込み中は、タブの表示を切り替えられない")
}

/// What OK does to the windows: the bar and the grouping are switched for every window at once,
/// as `F_SHOWTAB` and `F_BIND_WINDOW` do (`ResetGroupId` and a message to all windows).
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, before: Option<&Values>) {
    env.cx.set_global(options(after));
    let (show_bar, grouped, new_window) = (
        after.bool("tab_bar"),
        after.bool("tab_grouped"),
        after.bool("tab_open_new_win"),
    );
    let change = move |s: &mut settings::TabSettings| {
        s.show_bar = show_bar;
        s.grouped = grouped;
        s.new_window = new_window;
    };
    if before.is_none() {
        // The application starts with these.
        return;
    }
    let now = settings::current(env.cx);
    if now.show_bar == show_bar && now.grouped == grouped {
        settings::update(env.cx, change);
        return;
    }
    // The windows gather or spread from the window the dialog was opened from.
    let opener: Option<AnyWindowHandle> = env.opener.filter(|h| env.cx.windows().contains(h));
    let handle = opener.or_else(|| {
        crate::file_ops::windows::editors(env.cx)
            .into_iter()
            .next()
            .map(|(handle, _)| handle)
    });
    match handle {
        Some(handle) => {
            let done = handle.update(env.cx, |_, window, cx| {
                flows::change_settings(cx, window, change);
            });
            if done.is_err() {
                settings::update(env.cx, change);
            }
        }
        None => settings::update(env.cx, change),
    }
}
