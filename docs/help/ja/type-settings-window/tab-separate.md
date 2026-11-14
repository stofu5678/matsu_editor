# 新規グループ

いまのタブを、新しいウィンドウに移します。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > タブの操作 > 新規グループ |
| 機能名 | `F_TAB_SEPARATE` |
| 既定のキー | なし |
| マクロ | `TabSeparate` |
| キーマクロに記録 | しない |

## 動き

- タブモード（タブバーを表示していて、ウィンドウをまとめる設定のとき。共通設定の「タブバー」ページで決めます）のときだけ働きます。
- ウィンドウにタブが 1 つしかないときは、何も起きません（分ける相手がありません）。
- ファイルを読み込み中のウィンドウでは、何も起きません（読み込みの処理は、そのウィンドウのものです）。
- 移したタブは、分割した区画、カーソル、元に戻す履歴ごと、新しいウィンドウに移ります。
- 新しいウィンドウを開けなかったときは、タブは元の場所に戻ります。

## サクラエディタとの違い

- サクラのグループは、MATSU Editor では 1 つのウィンドウに当たります。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_TAB_SEPARATE`、`sakura_core/window/CTabWnd.cpp` の `Separate`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/tab_separate.rs`、`crates/matsu-ui/src/tabs/flows.rs`（`separate_tab`）
