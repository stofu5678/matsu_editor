//! 「タブバー」: `IDD_PROP_TAB` (`sakura_core/sakura_rc.rc`), `CPropTab`
//! (`sakura_core/prop/CPropComTab.cpp`).
//!
//! Live: the bar (`bDispTabWnd`), the grouping (`bDispTabWndMultiWin`, the opposite), the new
//! window for a file from outside (`bNewWindow`), what happens when the last file closes and when
//! the close button of the window is used (`bTab_RetainEmptyWin`, `bTab_CloseOneWin`), the width of
//! the tabs (`bSameTabWidth`), their close buttons (`bDispTabClose`) and the wheel over the bar
//! (`bChgWndByWheel`), the sorting of the tab list (`bSortTabList`) and the caption of the tabs
//! (`szTabWndCaption`, expanded like the title of the window), and the font of the tabs
//! (`lfTabFont`, with the font dialog), the icons (`bDispTabIcon`), the rows (`bTabMultiLine`) and the
//! place of the bar (`eTabPosition`). The widths of the tabs (`nTabMaxWidth`, `nTabMinWidth`,
//! `nTabMinWidthOnMulti`) have no control in Sakura's dialog; they are read from the file.
//!
//! `EnableTabPropInput` (`CPropComTab.cpp`) says which controls can be used: all of them need the
//! bar; the last three of the mode need the grouping too.

use gpui::AnyWindowHandle;
use matsu_config::settings::FontSetting;

use super::super::CommonSettingsDialog;
use super::super::Env;
use super::super::model::{Control, Item, Values};
use crate::font_dialog::model::{Choice, Style, face_fits};
use crate::tabs::caption::{self, TabCaption};
use crate::tabs::flows;
use crate::tabs::settings::{self, DispTabClose, TabFont, TabOptions, TabPosition};

/// `DispTabCloseArr` (`CPropComTab.cpp`).
const DISP_CLOSE: &[(&str, i32)] = &[("なし", 0), ("常に表示", 1), ("自動表示", 2)];
/// `TabPosArr`: the tab place (`TabPosition_Top`, `TabPosition_Bottom`).
const POSITION: &[(&str, i32)] = &[("上", 0), ("下", 1)];

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
                    Control::button("tab_font", "フォント(&F)...", choose_tab_font)
                        .when(bar_is_shown)
                        .into(),
                    Item::Custom(tab_font_label),
                    Item::Hidden(vec![Control::font(
                        TAB_FONT,
                        "lfTabFont",
                        Some("lfTabFontPs"),
                        Some("lfTabFaceName"),
                    )]),
                ]),
                Item::Row(vec![
                    Control::check(
                        "tab_sort_list",
                        "タブ一覧をソートする(&S)",
                        "bSortTabList",
                        true,
                    )
                    .when(bar_is_shown)
                    .into(),
                    Control::check("tab_multi_line", "多段(&M)", "bTabMultiLine", false)
                        .when(bar_is_shown)
                        .into(),
                    Control::choice("tab_position", "表示位置(&P)", "eTabPosition", 0, POSITION)
                        .when(bar_is_shown)
                        .into(),
                ]),
                Control::text(
                    "tab_caption",
                    "タイトル(&T)",
                    "szTabWndCaption",
                    caption::DEFAULT,
                )
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

/// The font of the tabs (`m_sTabBar.m_lf`, `m_nPointSize`): not drawn, the button and the label use it.
const TAB_FONT: &str = "tab_font_value";

/// `IDC_BUTTON_TABFONT` (`CPropTab::DispatchEvent`): the font dialog starts with the font of the tabs;
/// when the file has none, with the font of the program's windows at 9 points (Sakura's default is the
/// font of icon titles, `lfIconTitle`, which this system does not have).
fn choose_tab_font(
    dialog: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    let font = dialog.values().font(TAB_FONT);
    let initial = if font.log_font.face_name.is_empty() {
        let family = gpui_component::Theme::global(cx).font_family.to_string();
        Choice {
            family: if face_fits(&family) {
                family
            } else {
                String::new()
            },
            style: Style::Regular,
            size: 90,
        }
        .setting(&FontSetting::default())
    } else {
        font
    };
    dialog.choose_font(TAB_FONT, initial, window, cx);
}

/// `IDC_STATIC_TABFONT`: the face and the size of the font of the tabs.
fn tab_font_label(
    dialog: &CommonSettingsDialog,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) -> gpui::AnyElement {
    let font = dialog.values().font(TAB_FONT);
    crate::common_settings::model::font_label(Some(&font), cx)
}

/// The options of the tabs, from the values.
fn options(values: &Values, current: TabOptions) -> TabOptions {
    TabOptions {
        retain_empty: values.bool("tab_retain_empty"),
        close_one_win: values.bool("tab_close_one_win"),
        same_width: values.bool("tab_same_width"),
        disp_close: DispTabClose::from_code(values.int("tab_disp_close")),
        change_by_wheel: values.bool("tab_change_by_wheel"),
        sort_list: values.bool("tab_sort_list"),
        // The list switches this itself; the page does not have it.
        list_full: false,
        disp_icon: values.bool("tab_icon"),
        multi_line: values.bool("tab_multi_line"),
        position: TabPosition::from_code(values.int("tab_position")),
        // Sakura's dialog has no control for the widths: they stay as the file has them.
        max_width: current.max_width,
        min_width: current.min_width,
        min_width_multi: current.min_width_multi,
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
    let current = settings::options(env.cx);
    let mut tab_options = options(after, current);
    tab_options.list_full = current.list_full;
    // The caption and the font first: the windows draw again when the options are set.
    env.cx
        .set_global(TabCaption(after.text("tab_caption").to_owned()));
    let font = after.font(TAB_FONT);
    env.cx.set_global(TabFont(
        (!font.log_font.face_name.is_empty()).then_some(font),
    ));
    env.cx.set_global(tab_options);
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
