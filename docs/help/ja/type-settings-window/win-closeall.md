# すべて閉じる

開いているすべての文書を、1 つずつ閉じます。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > すべて閉じる |
| 機能名 | `F_WIN_CLOSEALL` |
| 既定のキー | なし |
| マクロ | `WinCloseAll` |
| キーマクロに記録 | しない |

## 動き

- 変更を保存していない文書は、「保存しますか」と聞きます。「キャンセル」を選ぶと、そこで止まります。それまでに閉じた文書は閉じたままで、残りは残ります。
- コマンドを実行しているウィンドウの文書は、最後に閉じます。実行している文書を先に閉じると、処理も一緒に終わってしまうためです。
- 実行中のシェルがあるウィンドウは、閉じる前に 1 度まとめて確認します。
- 共通設定「全般」の「同時に複数の編集用ウィンドウを閉じるとき確認」（`bCloseAllConfirm`、既定はオフ）がオンで、閉じる文書が 2 つ以上のときは、最初に「同時に複数の編集用ウィンドウを閉じようとしています。これらを閉じますか?」と聞きます。
- 最後のウィンドウを閉じると、MATSU Editor が終わります。

タブモード（タブバーを表示していて、ウィンドウをまとめる設定のとき。共通設定の「タブバー」ページで決めます）では、まとめたウィンドウのタブが対象です。そうでないときは、すべてのウィンドウの文書が対象です。

## サクラエディタとの違い

- サクラはタスクトレイの常駐プロセスが、閉じる処理を続けます。MATSU Editor に常駐プロセスはなく、最後のウィンドウを閉じると終わります。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_FILECLOSEALL`、`sakura_core/env/CAppNodeManager.cpp` の `RequestCloseEditor`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/win_closeall.rs`、`crates/matsu-ui/src/window_ops/mod.rs`（`close_windows`、`confirm_closing_several`）
- 試験: `crates/matsu-ui/src/window_ops/tests.rs`
