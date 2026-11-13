# 行の二重化(折り返し単位)

カーソルのある行を複製して、その上に入れます。

## 呼び出し方

- メニュー: 「編集」→「高度な操作」→「行の二重化(折り返し単位)」
- キー: `Ctrl+I`、`Shift+F10`
- マクロ: `DuplicateLine()`
- 機能名: `F_DUPLICATELINE`

## 動き

- カーソルのある画面の 1 行を複製して、その行の先頭に入れます。改行が付くのは、その行が文書の行の最後の画面の行のときです。折り返された行の途中の行は、その行の文字だけを複製し、改行は付けません。
- カーソルは、元の行の同じ位置に残ります。つまり、複製の 1 行下に動いたことになります。
- 改行のない最後の行では、複製のほうに、Enter で入る改行が付きます。
- 選択は、最初に外されます。
- 最後に改行がある文書の、その後ろの空の行にカーソルがあるとき、または空の文書では、複製する行がないので、警告音が鳴ります（このときも選択は外れます）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Edit_word_line.cpp` の `Command_DUPLICATELINE`
- MATSU のコード: `crates/matsu-commands/src/commands/edit/duplicate_line.rs`
- 試験: 同じファイルの `tests` モジュール
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`I`、`F10` の行）
