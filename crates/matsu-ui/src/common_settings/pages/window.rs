//! 「ウィンドウ」: `IDD_PROP_WIN` (`sakura_core/sakura_rc.rc`), `CPropWin`
//! (`sakura_core/prop/CPropComWin.cpp`).
//!
//! Live: the tool bar (`bDispTOOLBAR`), the function key bar (`bDispFUNCKEYWND`, its place and the
//! group size), the status bar (`bDispSTATUSBAR`) and the synchronized scrolling of the split panes
//! (`bSplitterWndVScroll`, `bSplitterWndHScroll`), the icons in menus (`bMenuIcon`) and the title
//! formats (`szWinCaptionActive`, `szWinCaptionInactive`): the title of a window is the format
//! expanded by `ExpandParameter` ([`crate::file_names::title`]). Not available yet: the horizontal
//! scroll bar, the dark mode, the size and position of the window, the ruler and the line number
//! gaps, the function keys and the language.

use gpui::{App, Global};

use super::super::Env;
use super::super::model::{Control, Item, Values};
use crate::file_names::title::{CAPTION_ACTIVE, CAPTION_INACTIVE, TitleFormats};

/// `IDC_SPIN_nRulerHeight_MIN` and `_MAX` (`sakura_core/_main/global.h`).
const RULER_HEIGHT: (i32, i32) = (2, 32);

/// What the split panes keep in step (`m_bSplitterWndVScroll`, `m_bSplitterWndHScroll`; both on
/// by default).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SplitterSync {
    pub vertical: bool,
    pub horizontal: bool,
}

impl Default for SplitterSync {
    fn default() -> Self {
        Self {
            vertical: true,
            horizontal: true,
        }
    }
}

impl Global for SplitterSync {}

/// The synchronized scrolling now.
pub(crate) fn splitter_sync(cx: &App) -> SplitterSync {
    cx.try_global::<SplitterSync>().copied().unwrap_or_default()
}

fn funckey_bar_is_shown(values: &Values) -> bool {
    values.bool("disp_funckey_wnd")
}

pub(in crate::common_settings) fn items() -> Vec<Item> {
    let (ruler_min, ruler_max) = RULER_HEIGHT;
    vec![
        Item::Group(
            "基本設定",
            vec![Item::Columns(vec![
                vec![
                    Control::check("disp_toolbar", "ツールバー表示(&T)", "bDispTOOLBAR", true)
                        .into(),
                    Control::check(
                        "disp_funckey_wnd",
                        "ファンクションキー表示(&K)",
                        "bDispFUNCKEYWND",
                        false,
                    )
                    .into(),
                    Control::check(
                        "disp_status_bar",
                        "ステータスバー表示(&S)",
                        "bDispSTATUSBAR",
                        true,
                    )
                    .into(),
                ],
                vec![
                    Control::check(
                        "scroll_bar_horz",
                        "水平スクロールバー(&R)",
                        "bScrollBarHorz",
                        true,
                    )
                    .unavailable()
                    .into(),
                    Control::check("dark_mode", "ダークモード(&D)", "bDarkMode", false)
                        .unavailable()
                        .into(),
                    Item::Label("タブバー表示は\n[タブバー]タブにあります"),
                ],
                vec![
                    Control::check("menu_icon", "アイコン付きメニュー(&I)", "bMenuIcon", true)
                        .into(),
                    Control::button("win_size", "位置と大きさの設定(&W)...", |_, _, _| {})
                        .unavailable()
                        .into(),
                ],
            ])],
        ),
        Item::Columns(vec![
            vec![
                Item::Group(
                    "ルーラー / 行番号",
                    vec![
                        Control::number(
                            "ruler_height",
                            "ルーラーの高さ(&E)",
                            "nRulerHeight",
                            13,
                            ruler_min,
                            ruler_max,
                        )
                        .unavailable()
                        .into(),
                        Control::number(
                            "ruler_bottom_space",
                            "ルーラーとテキストの隙間(&P)",
                            "nRulerBottomSpace",
                            0,
                            0,
                            32,
                        )
                        .unavailable()
                        .into(),
                        Control::number(
                            "line_number_right_space",
                            "行番号とテキストの隙間(&L)",
                            "nLineNumberRightSpace",
                            0,
                            0,
                            32,
                        )
                        .unavailable()
                        .into(),
                    ],
                ),
                Item::Group(
                    "分割ウィンドウ",
                    vec![
                        Control::check(
                            "splitter_v_scroll",
                            "垂直スクロールの同期をとる(&V)",
                            "bSplitterWndVScroll",
                            true,
                        )
                        .into(),
                        Control::check(
                            "splitter_h_scroll",
                            "水平スクロールの同期をとる(&H)",
                            "bSplitterWndHScroll",
                            true,
                        )
                        .into(),
                    ],
                ),
            ],
            vec![
                Item::Group(
                    "ファンクションキー",
                    vec![
                        Item::Row(vec![
                            Control::radio("funckey_place", "上(&O)", "nFUNCKEYWND_Place", 0, 1)
                                .when(funckey_bar_is_shown)
                                .into(),
                            Control::radio("funckey_place", "下(&B)", "nFUNCKEYWND_Place", 1, 1)
                                .when(funckey_bar_is_shown)
                                .into(),
                        ]),
                        Control::number(
                            "funckey_group_num",
                            "グループボタン数(&G)",
                            "nFUNCKEYWND_GroupNum",
                            4,
                            1,
                            12,
                        )
                        .when(funckey_bar_is_shown)
                        .into(),
                    ],
                ),
                Item::Group(
                    "言語",
                    vec![
                        Control::choice("language", "", "", 0, &[("日本語", 0)])
                            .named("言語")
                            .unavailable()
                            .into(),
                    ],
                ),
            ],
        ]),
        Item::Group(
            "タイトルバー",
            vec![
                Control::text(
                    "win_caption_active",
                    "アクティブ時(&1)",
                    "szWinCaptionActive",
                    CAPTION_ACTIVE,
                )
                .into(),
                Control::text(
                    "win_caption_inactive",
                    "非アクティブ時(&2)",
                    "szWinCaptionInactive",
                    CAPTION_INACTIVE,
                )
                .into(),
            ],
        ),
    ]
}

/// What OK does to the windows: every window takes the status bar, the toolbar and the function
/// key bar of the check boxes (`CEditWnd::OnChangeSetting`), and the panes take the synchronized
/// scrolling.
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, before: Option<&Values>) {
    env.cx.set_global(TitleFormats {
        active: after.text("win_caption_active").to_owned(),
        inactive: after.text("win_caption_inactive").to_owned(),
    });
    env.cx.set_global(SplitterSync {
        vertical: after.bool("splitter_v_scroll"),
        horizontal: after.bool("splitter_h_scroll"),
    });
    // The menus that open from now on show the icons or not (`m_bMenuIcon`).
    crate::menu_icons::set(env.cx, after.bool("menu_icon"));
    // The native menu of macOS is made once and keeps its pictures: make it again.
    if cfg!(target_os = "macos") {
        crate::app::refresh_menus(env.cx);
    }
    // At startup the bars take the file as it is (`crate::bars::install`); OK takes the dialog.
    if before.is_some() {
        crate::bars::set(
            env.cx,
            crate::bars::BarSettings {
                toolbar: after.bool("disp_toolbar"),
                funckeys: after.bool("disp_funckey_wnd"),
                funckey_place: if after.int("funckey_place") == 1 {
                    crate::bars::FuncKeyPlace::Bottom
                } else {
                    crate::bars::FuncKeyPlace::Top
                },
                funckey_group_num: after.int("funckey_group_num").clamp(1, 12) as usize,
            },
        );
    }
    let shown = after.bool("disp_status_bar");
    if before.is_some_and(|b| b.bool("disp_status_bar") != shown) {
        super::super::windows::set_status_bar(env.cx, shown);
    }
}
