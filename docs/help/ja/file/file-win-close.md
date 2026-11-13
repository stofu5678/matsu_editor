# 閉じる

この文書を閉じます。

- 機能名: `F_WINCLOSE`
- メニュー: 「ファイル」の「閉じる」
- 既定のキー: なし（`key_bindings.rs` の既定の割り当てにありません）
- マクロ: `WinClose()`

## 動き

1. 文書に変更があると、「保存しますか」と聞きます（[章の初めの説明](index.md)を見てください）
2. 閉じてよければ、閉じます。タブを使っていて、ほかのタブがあるときは、このタブだけを閉じます。最後の 1 つなら、ウィンドウを閉じます
3. 「キャンセル」を選ぶか、保存をやめたときは、何も閉じません

名前のない文書を残したいときは、[閉じて(無題)](file-close.md)を使います。

## サクラエディタとの違い

動きはサクラエディタと同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_WINCLOSE`、`sakura_core/doc/CEditDoc.cpp` の `OnFileClose`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/win_close.rs`、`crates/matsu-ui/src/file_ops/close.rs` の `confirm_and_close_window`
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`（この機能のコード 31320 がない）
