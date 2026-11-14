# 行の切り取りと削除

行を単位にして、切り取ったり消したりする 6 つの操作です。メニューは「編集」→「高度な操作」にあります。

| 機能名 | メニュー | キー | マクロ | 単位 |
|---|---|---|---|---|
| `F_LineCutToStart` | 行頭まで切り取り(改行単位) | `Ctrl+U` | `LineCutToStart()` | 改行まで |
| `F_LineCutToEnd` | 行末まで切り取り(改行単位) | `Ctrl+K` | `LineCutToEnd()` | 改行まで |
| `F_LineDeleteToStart` | 行頭まで削除(改行単位) | `Ctrl+Shift+U` | `LineDeleteToStart()` | 改行まで |
| `F_LineDeleteToEnd` | 行末まで削除(改行単位) | `Ctrl+Shift+K` | `LineDeleteToEnd()` | 改行まで |
| `F_CUT_LINE` | 行切り取り(折り返し単位) | `Ctrl+E` | `CutLine()` | 画面の 1 行 |
| `F_DELETE_LINE` | 行削除(折り返し単位) | `Ctrl+Shift+E` | `DeleteLine()` | 画面の 1 行 |

「改行単位」は、文書の行（次の改行まで）です。折り返し表示でも、行頭は最初の行の先頭、行末は最後の行の終わりです。「折り返し単位」は、画面に見えている 1 行です。

## 行頭まで・行末まで（切り取り / 削除）

- 「行頭まで」は、行の先頭からカーソルまで、「行末まで」は、カーソルから行の終わり（改行の前）までです。改行は残ります。
- 選択があるときは、その選択を切り取る（または消す）だけです。
- 行頭で「行頭まで」を、行末で「行末まで」を使うと、警告音が鳴ります。「行頭まで削除」「行末まで削除」は、最後に改行がある文書の、その後ろの空の行でも警告音が鳴ります。
- 「切り取り」は、範囲をクリップボードに写します。「削除」は写しません。

## 行切り取りと行削除

- カーソルのある画面の 1 行を、改行といっしょに切り取ります（行削除は消すだけで、クリップボードは変わりません）。
- 「行切り取り」で切った行は、「行コピー」として覚えられます。貼り付けると行頭に入ります（[貼り付け](paste.md)）。
- 選択があるとき、または、最後に改行がある文書の、その後ろの空の行にいるときは、警告音が鳴るだけです。
- 行を消したあとのカーソルは、その場所に上がってきた行の、同じ表示桁に行きます。その行が短いときは、行の終わりに行きます。

## サクラエディタとの違い

「行削除」で、改行のない最後の行を消したあと、サクラエディタはカーソルを行末の 1 つ手前に置きます。MATSU Editor は、行末に置きます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Edit_word_line.cpp` の `Command_LineCutToStart`、`Command_LineCutToEnd`、`Command_LineDeleteToStart`、`Command_LineDeleteToEnd`、`Command_CUT_LINE`、`Command_DELETE_LINE`
- MATSU のコード: `crates/matsu-commands/src/commands/clipboard/line_cut_to_start.rs`、`line_cut_to_end.rs`、`cut_line.rs`、`crates/matsu-commands/src/commands/edit/line_delete_to_start.rs`、`line_delete_to_end.rs`、`delete_line.rs`（先頭のコメント）
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`U`、`K`、`E` の行）。macOS の `Cmd+Backspace` は `docs/adr/0008-commands-and-keybindings.md`
