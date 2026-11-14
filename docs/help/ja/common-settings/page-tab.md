# 共通設定: タブバー

タブバーの表示と、タブの動きを決めます。キーは `[Common]` セクションです。

「タブバーを表示する」がオフのときは、ほかの項目は使えません。「動作モード」の下の 3 つは、「ウィンドウをまとめてグループ化する」もオンのときだけ使えます。

| 項目 | キー | 既定 | 動き |
|---|---|---|---|
| タブバーを表示する | `bDispTabWnd` | オフ | タブバーを出します |
| ウィンドウをまとめてグループ化する | `bDispTabWndMultiWin` | （値は逆） | オンのとき、ウィンドウが 1 つにまとまり、タブで切り替えます。キーの値は、チェックと逆です（チェックがオンなら、キーは 0） |
| 最後のファイルを閉じたとき(無題)文書を残す | `bTab_RetainEmptyWin` | オン | 最後のタブを閉じても、空のウィンドウを残します |
| ウィンドウの閉じるボタンは現在のファイルのみ閉じる | `bTab_CloseOneWin` | オフ | ウィンドウの閉じるボタンで、いまのタブだけを閉じます |
| 外部から起動するときは新しいウインドウで開く | `bNewWindow` | オフ | ほかのところから開くファイルは、新しいウィンドウに開きます |
| アイコン表示 | `bDispTabIcon` | オフ | タブにアイコンを出します |
| 等幅 | `bSameTabWidth` | オフ | タブの幅をそろえます |
| 閉じるボタン | `bDispTabClose` | なし | 「なし」「常に表示」「自動表示」から選びます |
| フォント... | `lfTabFont` ほか | 未設定 | タブのフォントを、フォントのダイアログで選びます |
| タブ一覧をソートする | `bSortTabList` | オン | タブの一覧を並べ替えます |
| 多段 | `bTabMultiLine` | オフ | タブを何段かに分けて並べます |
| 表示位置 | `eTabPosition` | 上 | 上か下から選びます |
| タイトル | `szTabWndCaption` | コードの既定の書式（`tabs::caption::DEFAULT`） | タブに出す文字の書式 |
| マウスホイールでタブ切替 | `bChgWndByWheel` | オフ | タブバーの上でホイールを回すと、タブを切り替えます |

- 「タイトル」の書式は、ウィンドウのタイトルと同じように展開されます（[ウィンドウ](page-window.md) ページ）
- タブの幅の上限と下限（`nTabMaxWidth`、`nTabMinWidth`、`nTabMinWidthOnMulti`）は、ダイアログにありません。ファイルから読みます

## 根拠

- サクラのソース: `sakura_core/prop/CPropComTab.cpp` の `EnableTabPropInput`（どの部品が使えるか）
- MATSU のコード: `crates/matsu-ui/src/common_settings/pages/tab.rs`、`crates/matsu-ui/src/tabs/caption.rs`（`DEFAULT`）、`crates/matsu-ui/src/tabs/settings.rs`
