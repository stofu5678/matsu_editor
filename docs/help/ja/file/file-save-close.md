# 保存して閉じる

保存してから、この文書を閉じます。

- 機能名: `F_FILESAVECLOSE`
- メニュー: 「ファイル」の「保存して閉じる」
- 既定のキー: なし
- マクロ名: なし（`macro_functions.rs` の表に載っていません）

## 動き

1. [上書き保存](file-save.md)と同じに保存します。ファイルがない文書は、名前を聞きます
2. 保存できたら、閉じます。タブを使っていて、ほかのタブがあるときは、このタブだけを閉じます。最後の 1 つなら、ウィンドウを閉じます
3. 保存をやめた（ダイアログのキャンセルなど）ときは、閉じません

変更がない文書は、保存せずに、閉じるだけです。

## サクラエディタとの違い

動きはサクラエディタと同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander.cpp` の `case F_FILESAVECLOSE`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/file_save_close.rs`、`crates/matsu-ui/src/file_ops/close.rs` の `save_and_close_window`
