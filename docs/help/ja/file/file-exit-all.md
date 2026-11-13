# マツエディタの全終了

開いているウィンドウをすべて閉じて、アプリケーションを終了します。

- 機能名: `F_EXITALL`
- メニュー: 「ファイル」の「マツエディタの全終了」（サクラエディタでは「サクラエディタの全終了」）
- 既定のキー: `Ctrl+Alt+F4`
- マクロ: `ExitAll()`

## 動き

1. 共通設定「全般」の「マツエディタの全終了で編集用ウィンドウを閉じるとき確認」が入っていて、ウィンドウが開いているときは、「現在開いている編集用のウィンドウをすべて閉じて終了しますか?」と聞きます。「いいえ」なら、何も閉じません。この確認を出したときは、ウィンドウの数についての確認は出しません
2. 変更のある文書を、1 つずつ「保存しますか」と聞きます（[章の初めの説明](index.md)を見てください）
3. 「キャンセル」を選ぶと、全体が止まります。何も閉じません
4. シェルが動いているときは、1 回だけ確認します
5. すべて答えたら、プラグインのイベントを呼んで、閉じて終了します

手順 2 以降は、[編集の全終了](file-exit-all-editors.md)と同じです。

## サクラエディタとの違い

[編集の全終了](file-exit-all-editors.md)と同じです。サクラエディタでは、この機能が常駐のトレイのプロセスまで終わらせますが、MATSU Editor にはそのプロセスがないので、2 つの違いは確認の設定だけです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_EXITALL`、`sakura_core/_main/CControlTray.cpp` の `TerminateApplication`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/exit_all.rs`、`crates/matsu-ui/src/file_ops/close.rs` の `exit_all`・`ExitKind::Application`
- 設定: `bExitConfirm`（`crates/matsu-ui/src/common_settings/pages/general.rs`）
- メニューの名前: `crates/matsu-commands/src/sakura/main_menu.rs`
