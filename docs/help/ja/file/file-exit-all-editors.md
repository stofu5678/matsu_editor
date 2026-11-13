# 編集の全終了

開いているウィンドウをすべて閉じて、終了します。

- 機能名: `F_EXITALLEDITORS`
- メニュー: 「ファイル」の「編集の全終了」
- 既定のキー: `Alt+Shift+F4`
- マクロ: `ExitAllEditors()`

## 動き

1. 複数のウィンドウが開いていて、共通設定「全般」の「同時に複数の編集用ウィンドウを閉じるとき確認」が入っているときは、「同時に複数の編集用ウィンドウを閉じようとしています。これらを閉じますか?」と聞きます。「いいえ」なら、何も閉じません
2. 変更のある文書を、1 つずつ「保存しますか」と聞きます（[章の初めの説明](index.md)を見てください）
3. 「キャンセル」を選ぶと、全体が止まります。何も閉じません
4. シェルが動いているときは、終了するとシェルも終わるので、1 回だけ確認します
5. すべて答えたら、プラグインの `DocumentClose` と `EditorEnd` のイベントを呼んで、閉じて終了します

確認している間も、ほかのウィンドウは使えます。確認したあとに文書が変わったり、ウィンドウが増えたりしたときは、閉じる前にもう一度確認します。変更を、聞かずに捨てることはありません。

[マツエディタの全終了](file-exit-all.md)との違いは、手順 1 の確認の設定だけです。

## サクラエディタとの違い

サクラエディタには、ウィンドウとは別に、タスクトレイに常駐するプロセスがあります。サクラエディタの「編集の全終了」は、ウィンドウだけを閉じて、トレイのプロセスは残します。MATSU Editor にはトレイのプロセスがないので、ウィンドウを閉じると、アプリケーションも終わります。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_EXITALLEDITORS`、`sakura_core/_main/CControlTray.cpp` の `CloseAllEditor`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/exit_all_editors.rs`、`crates/matsu-ui/src/file_ops/close.rs` の `exit_all`・`ExitKind`、`crates/matsu-ui/src/window_ops/mod.rs` の `confirm_closing_several`
- 設定: `bCloseAllConfirm`（`crates/matsu-ui/src/common_settings/pages/general.rs`）
- 差の記録: `docs/sakura-gaps.md`（タスクトレイの行）
