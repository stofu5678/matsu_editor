# このタブ／ウィンドウ以外を閉じる

いまの文書を残して、ほかの文書をすべて閉じます。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > このタブ／ウィンドウ以外を閉じる |
| 機能名 | `F_TAB_CLOSEOTHER` |
| 既定のキー | なし |
| マクロ | `TabCloseOther` |
| キーマクロに記録 | しない |

## 動き

- 保存の確認、「キャンセル」で止まること、複数を閉じるときの確認は、「すべて閉じる」と同じです。違いは、コマンドを実行しているウィンドウの文書を閉じないことです。
- 実行中のシェルの確認は、このコマンドでは行いません。
- タブモード（タブバーを表示していて、ウィンドウをまとめる設定のとき。共通設定の「タブバー」ページで決めます）では、ほかのタブが閉じて、いまのタブが残ります。そうでないときは、ほかのウィンドウが閉じます。
- 文書を持たないウィンドウ（プレビューやシェルだけのウィンドウ）は、このコマンドでは閉じません。

## サクラエディタとの違い

- サクラの動きに合わせています（`RequestCloseEditor` を、いまのウィンドウを除く各ウィンドウに送る）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_TAB_CLOSEOTHER`、`sakura_core/env/CAppNodeManager.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/tab_closeother.rs`、`crates/matsu-ui/src/window_ops/mod.rs`（`close_windows` の `keep_current`）
- 試験: `crates/matsu-ui/src/window_ops/tests.rs`
