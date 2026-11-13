# 新規作成

名前のない新しい文書を作ります。

- 機能名: `F_FILENEW`
- メニュー: 「ファイル」の「新規作成」
- 既定のキー: `Ctrl+N`
- マクロ: `FileNew()`

## 動き

- タブを使っている（タブバーを出し、ウィンドウをグループにまとめている）ときは、同じウィンドウに新しいタブができます
- それ以外のときは、新しいウィンドウができます
- 今の文書には、何もしません。変更の確認も出ません

ウィンドウを必ず新しく作りたいときは、[新規ウインドウを開く](file-new-newwindow.md)を使います。

## サクラエディタとの違い

動きはサクラエディタと同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_FILENEW`、`sakura_core/_main/CControlTray.cpp` の `OpenNewEditor`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/file_new.rs`
- マクロ名: `crates/matsu-commands/src/sakura/macro_functions.rs`
