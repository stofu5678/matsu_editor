# 閉じて開く

今のファイルを閉じて、選んだファイルを同じウィンドウに開きます。

- 機能名: `F_FILECLOSE_OPEN`
- メニュー: 「ファイル」の「閉じて開く...」
- 既定のキー: `Ctrl+Shift+F4`
- マクロ: `FileCloseOpen()`

## 動き

1. 文書に変更があると、「保存しますか」と聞きます（[章の初めの説明](index.md)を見てください）。「キャンセル」を選ぶと、何も変わりません
2. 「開く」のダイアログが出ます
3. ダイアログをキャンセルすると、今のファイルがそのまま残ります
4. 1 つ目のファイルは、このウィンドウに開きます。すでにほかのウィンドウで開いているファイルなら、そのウィンドウを前に出します
5. 複数のファイルを選んだときは、2 つ目からは新しいウィンドウに開きます

[開く](file-open.md)とのちがいは、今の文書を先に閉じることです。

## サクラエディタとの違い

動きはサクラエディタと同じです。

## 根拠

- サクラのソース: `sakura_core/doc/CDocFileOperation.cpp` の `FileCloseOpen`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/file_close_open.rs`、`crates/matsu-ui/src/file_ops/close.rs` の `close_and_open`
