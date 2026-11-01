//! 「ウィンドウ」: `IDD_PROP_WIN` (`sakura_core/sakura_rc.rc`), `CPropWin`
//! (`sakura_core/prop/CPropComWin.cpp`).
//!
//! Live: the tool bar (`bDispTOOLBAR`), the function key bar (`bDispFUNCKEYWND`, its place and the
//! group size), the status bar (`bDispSTATUSBAR`) and the synchronized scrolling of the split panes
//! (`bSplitterWndVScroll`, `bSplitterWndHScroll`), the icons in menus (`bMenuIcon`) and the title
//! formats (`szWinCaptionActive`, `szWinCaptionInactive`): the title of a window is the format
//! expanded by `ExpandParameter` ([`crate::file_names::title`]), the height of the ruler and the
//! gaps under it and after the line numbers (`nRulerHeight`, `nRulerBottomSpace`,
//! `nLineNumberRightSpace`, read with the rest of the view settings). The button 位置と大きさの設定
//! opens [`crate::win_size_dialog`], which edits the seven keys of the size and position of a new
//! window ([`crate::saved_state::window`]); the page keeps them in controls that are not drawn.
//! Not available yet: the horizontal scroll bar (the view has none), the dark mode, the function
//! keys and the language.

use gpui::{App, Global};

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::model::{Control, Item, Values};
use crate::file_names::title::{CAPTION_ACTIVE, CAPTION_INACTIVE, TitleFormats};
use crate::win_size_dialog::{self, WinSize};

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

/// The controls that hold the size and the position of a new window while the page is open:
/// the id, the key of `[Common]` and Sakura's default (`CShareData::InitShareData`; a width or
/// an X that was never stored is `CW_USEDEFAULT`).
const WIN_SIZE_KEYS: [(&str, &str, i32); 7] = [
    ("win_size_mode", "bSaveWindowSize", 1),
    ("win_size_type", "nWinSizeType", 0),
    ("win_size_cx", "nWinSizeCX", i32::MIN),
    ("win_size_cy", "nWinSizeCY", 0),
    ("win_pos_mode", "nSaveWindowPos", 0),
    ("win_pos_x", "nWinPosX", i32::MIN),
    ("win_pos_y", "nWinPosY", 0),
];

/// The values of the seven controls, as the dialog 位置と大きさの設定 gets them.
fn win_size_of(values: &Values) -> WinSize {
    WinSize {
        size_mode: values.int("win_size_mode"),
        size_type: values.int("win_size_type"),
        width: values.int("win_size_cx"),
        height: values.int("win_size_cy"),
        pos_mode: values.int("win_pos_mode"),
        x: values.int("win_pos_x"),
        y: values.int("win_pos_y"),
    }
}

/// `IDC_BUTTON_WINSIZE` (`CPropWin::DispatchEvent`): the dialog starts with the values of the
/// page and gives them back when it is closed. The page takes them as they are; the file is
/// written when OK is pressed.
fn open_win_size(
    dialog: &mut CommonSettingsDialog,
    _: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    let start = win_size_of(dialog.values());
    let this = cx.entity().downgrade();
    win_size_dialog::open(
        cx,
        start,
        Box::new(move |app: &mut App, now: WinSize| {
            this.update(app, |dialog, cx| {
                for (id, value) in [
                    ("win_size_mode", now.size_mode),
                    ("win_size_type", now.size_type),
                    ("win_size_cx", now.width),
                    ("win_size_cy", now.height),
                    ("win_pos_mode", now.pos_mode),
                    ("win_pos_x", now.x),
                    ("win_pos_y", now.y),
                ] {
                    dialog.set_int(id, value, cx);
                }
            })
            .ok();
        }),
    );
}

/// `values` for what OK writes: when the dialog 位置と大きさの設定 changed nothing, the size and
/// position that the windows have now (a window that was moved or resized while the sheet was
/// open, and the file written meanwhile), not the snapshot the sheet started with. A change made
/// in the dialog is kept as it is.
pub(in crate::common_settings) fn keep_current_geometry(
    cx: &App,
    before: &Values,
    after: &Values,
) -> Values {
    let mut stored = after.clone();
    if win_size_of(before) != win_size_of(after) {
        return stored;
    }
    let now = crate::saved_state::window::state(cx);
    for (id, value) in [
        ("win_size_mode", now.size_mode.code()),
        ("win_size_type", now.size_type),
        ("win_size_cx", now.width),
        ("win_size_cy", now.height),
        ("win_pos_mode", now.pos_mode.code()),
        ("win_pos_x", now.x),
        ("win_pos_y", now.y),
    ] {
        stored.set(id, super::super::model::Value::Int(value));
    }
    stored
}

/// OK of the sheet while the dialog 位置と大きさの設定 is open: the dialog closes with the sheet
/// and what it holds is taken, as if 閉じる had been pressed first.
pub(in crate::common_settings) fn absorb_win_size_dialog(
    dialog: &mut CommonSettingsDialog,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    if let Some(now) = win_size_dialog::take_open(cx) {
        for (id, value) in [
            ("win_size_mode", now.size_mode),
            ("win_size_type", now.size_type),
            ("win_size_cx", now.width),
            ("win_size_cy", now.height),
            ("win_pos_mode", now.pos_mode),
            ("win_pos_x", now.x),
            ("win_pos_y", now.y),
        ] {
            dialog.set_int(id, value, cx);
        }
    }
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
                    Control::button("win_size", "位置と大きさの設定(&W)...", open_win_size).into(),
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
                        .into(),
                        Control::number(
                            "ruler_bottom_space",
                            "ルーラーとテキストの隙間(&P)",
                            "nRulerBottomSpace",
                            0,
                            0,
                            32,
                        )
                        .into(),
                        Control::number(
                            "line_number_right_space",
                            "行番号とテキストの隙間(&L)",
                            "nLineNumberRightSpace",
                            0,
                            0,
                            32,
                        )
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
        // The size and the position of a new window, which the button 位置と大きさの設定 edits. A
        // choice with no entries holds a number the dialog does not clamp.
        Item::Hidden(
            WIN_SIZE_KEYS
                .into_iter()
                .map(|(id, key, default)| Control::choice(id, "", key, default, &[]))
                .collect(),
        ),
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
    // The windows that open from now on take the new size and position; those that are open keep
    // what they have. Nothing is touched when nothing here changed, so what a window moved to
    // while the dialog was open is not overwritten.
    if before.is_some_and(|b| win_size_of(b) != win_size_of(after)) {
        use crate::saved_state::window::SaveMode;
        let new = win_size_of(after);
        let mut state = crate::saved_state::window::state(env.cx);
        state.size_mode = SaveMode::from_code(new.size_mode);
        state.size_type = new.size_type;
        state.width = new.width;
        state.height = new.height;
        state.pos_mode = SaveMode::from_code(new.pos_mode);
        state.x = new.x;
        state.y = new.y;
        env.cx.set_global(state);
    }
    let shown = after.bool("disp_status_bar");
    if before.is_some_and(|b| b.bool("disp_status_bar") != shown) {
        super::super::windows::set_status_bar(env.cx, shown);
    }
}
