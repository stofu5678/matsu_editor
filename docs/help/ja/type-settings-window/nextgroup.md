# 次のグループ

次のグループ（ウィンドウ）を前面に出します。最後のグループから使うと、最初に戻ります。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > タブの操作 > 次のグループ |
| 機能名 | `F_NEXTGROUP` |
| 既定のキー | なし |
| マクロ | `NextGroup` |
| キーマクロに記録 | しない |

## 動き

- タブモード（タブバーを表示していて、ウィンドウをまとめる設定のとき。共通設定の「タブバー」ページで決めます）のときだけ働きます。そうでないときは何も起きません。
- ウィンドウは開いた順に数えます。ウィンドウが 1 つだけのときは何も起きません。
- ウィンドウの中のタブを移る「次のウィンドウ」とは別です。こちらはウィンドウを移ります。

## サクラエディタとの違い

- サクラのグループは、MATSU Editor では 1 つのウィンドウに当たります。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_NEXTGROUP`、`sakura_core/window/CTabWnd.cpp` の `NextGroup`、`GetNextGroupWnd`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/nextgroup.rs`、`crates/matsu-ui/src/tabs/flows.rs`（`go_to_group`）、`crates/matsu-ui/src/tabs/order.rs`（`next_group`）
