# 単語の削除と切り取り

カーソルのまわりの単語を消したり、切り取ったりする 4 つの操作です。メニューは「編集」→「高度な操作」にあります。

| 機能名 | メニュー | キー | マクロ |
|---|---|---|---|
| `F_WordDeleteToStart` | 単語の左端まで削除 | `Ctrl+BkSp` | `WordDeleteToStart()` |
| `F_WordDeleteToEnd` | 単語の右端まで削除 | `Ctrl+Del` | `WordDeleteToEnd()` |
| `F_WordCut` | 単語切り取り | `Ctrl+D` | `WordCut()` |
| `F_WordDelete` | 単語削除 | `Ctrl+Shift+D` | `WordDelete()` |

## 単語の左端まで削除

カーソルの前の単語の先頭から、カーソルまでを消します（[単語の左端に移動](cursor-moves.md)で行く位置までです）。

- 行頭では 1 文字ぶん左へ動くので、前の行の改行が消えます。
- 文書の先頭など、何も選ばれなかったときは、警告音が鳴ります。
- 選択があるときは、その選択を、カーソルの動いた先まで広げて消します。

## 単語の右端まで削除

カーソルから、次の単語の先頭までを消します。名前は「右端まで」ですが、動く先は次の単語の先頭なので、単語の後ろの空白も一緒に消えます。

- 行末では 1 文字ぶん右へ動くので、改行が消えます。
- 文書の終わりでは警告音が鳴ります。

## 単語切り取り

- 選択があるときは、[切り取り](cut.md)と同じです。
- 選択がないときは、カーソルの位置の単語を選んで切り取ります。
- 単語がない場所（行末など）では、次の 1 文字（行末なら改行）を選んで切り取ります。
- 文書の終わりでは、何も選べないので、選択なしの「切り取り」の動き（その行を切る）になります。

## 単語削除

- 選択があるときは、その範囲を消します。
- 選択がないときは、カーソルの位置の単語を消します。
- 単語がない場所（行末など）では、[削除](delete.md)と同じに、次の 1 文字を消します。行末なら次の行がつながります。

## 矩形選択のとき

「単語の左端まで削除」と「単語の右端まで削除」は、中身のある矩形選択では、警告音が鳴るだけで何も変わりません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Edit_word_line.cpp` の `Command_WordDeleteToStart`、`Command_WordDeleteToEnd`、`Command_WordCut`、`Command_WordDelete`
- MATSU のコード: `crates/matsu-commands/src/commands/edit/word_delete_to_start.rs`、`word_delete_to_end.rs`、`word_cut.rs`、`word_delete.rs`（先頭のコメント）
- 試験: 上の 4 つのファイルの `tests` モジュール
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`BkSp`、`Del`、`D` の行）。macOS の `Option+BkSp` と `Option+Del` の割り当ては `docs/adr/0008-commands-and-keybindings.md`
