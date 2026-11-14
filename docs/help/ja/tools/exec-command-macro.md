# 外部コマンド実行のマクロ（ExecCommand）

マクロから、ダイアログを出さずに外部コマンドを実行します。動きは [外部コマンド実行](exec-command.md) と同じです。違うのは、選択肢をダイアログのチェックではなく、数の和で渡すことです。

- マクロの名前: `ExecCommand`
- 機能: `F_EXECMD`
- キーマクロ: 記録されます。ダイアログから実行したときも、この呼び出しとして記録されます
- メニューとキー: ありません（メニューにあるのはダイアログの `F_EXECMD_DIALOG` です）

## 引数

```text
ExecCommand(command, option, folder)
```

| 引数 | 型 | 意味 |
|---|---|---|
| command | 文字列 | 実行するコマンドラインです。必ず要ります。`$F` と `$$` などは、ダイアログと同じに置き換えます |
| option | 数 | 選択肢の和です。省略すると 0 です（下の表） |
| folder | 文字列 | 動かすフォルダです。`option` に 0x200 が入っているときだけ使います。省略できます |

言語ごとの書き方は、「マクロの使い方」の章にあります。

`command` がないと、マクロはエラーで止まります。`option` が数でなければ、同じくエラーです。`option` が 32 ビットの数に収まらないときも、エラーです。

## option の和

| 値 | 意味 |
|---|---|
| 0x01 | 標準出力と標準エラーを受け取ります。これがないと、起動だけして戻ります |
| 0x02 | 出力を編集中の文書に入れます。ないときはアウトプットに入れます（文書が読み取り専用のときも、アウトプットです） |
| 0x04 | 選択している文字列（なければ文書全体）を標準入力に送ります |
| 0x08 | 出力を Unicode（UTF-16 LE）として読みます |
| 0x10 | 入力を Unicode（UTF-16 LE）として送ります |
| 0x20 | 前後の情報（`#====` の行、日時、コマンドライン、終了コード）を入れます |
| 0x40 | 前後の情報を入れません。0x20 と一緒に指定したときは、0x40 が勝ちます |
| 0x80 | 出力を UTF-8 として読みます |
| 0x100 | 入力を UTF-8 として送ります |
| 0x200 | `folder` のフォルダで動かします |

### 文字コードの決まり

- 出力は、0x08 があれば Unicode、なければ 0x80 があれば UTF-8、どちらもなければ Shift_JIS です。
- 入力は、0x10 があれば Unicode、なければ 0x100 があれば UTF-8、どちらもなければ Shift_JIS です。

### 前後の情報の決まり

- 出力がアウトプットに入るときは、前後の情報を入れます。
- 出力が文書に入るときは、入れません。
- 0x20 で、どちらでも入れられます。0x40 で、どちらでも止められます。

### MATSU Editor だけの 0x400

0x400 を足すと、出力を 1 行ずつ読みます。その行が UTF-8 として正しければ UTF-8 で、そうでなければ Shift_JIS で読みます。UTF-8 のクライアントと、Shift_JIS のメッセージが混ざる出力のためです。

ダイアログからは選べません。データベースの機能が使います。0x08 か 0x80 があると、そちらが勝ちます。サクラエディタにはない値です。

## 例

| option | 動き |
|---|---|
| 0 | 起動だけして戻ります。出力は受け取りません |
| 1 | 出力をアウトプットに出します。前後の情報つきです |
| 0x41（65） | 出力をアウトプットに出します。前後の情報はありません |
| 3 | 出力を文書のキャレットの位置に入れます |
| 7 | 選んだ範囲（なければ文書全体）をコマンドに通し、出力で置き換えます |
| 0x187（391） | 7 と同じで、入力も出力も UTF-8 です |

## 動きの補足

- マクロは、コマンドが終わるまで次の行に進みません（出力を受け取るとき）。0x01 がなければ、待ちません。
- 出力を受け取るコマンドは、1 つの窓で 1 つずつ動きます。
- 履歴は変わりません。履歴に入るのは、ダイアログから実行したものだけです。
- ほかの動き（選択の扱い、中断、元に戻す、標準入力）は、[外部コマンド実行](exec-command.md) のページのとおりです。

## サクラエディタとの違い

- 0x400 は、MATSU Editor だけです。
- そのほかの違いは、[外部コマンド実行](exec-command.md) の「サクラエディタとの違い」のとおりです。

## 根拠

- サクラのソース: `sakura_core/macro/CMacro.cpp` の `CMacro::HandleCommand`（`F_EXECMD` の引数の読み方）、`sakura_core/cmd/CViewCommander_Macro.cpp` の `Command_EXECCOMMAND`、`sakura_core/view/CEditView_ExecCmd.cpp` の `CEditView::ExecCmd`
- MATSU のコード:
  - `crates/matsu-ui/src/ui_commands/macros/execmd.rs`（引数と値の表）
  - `crates/matsu-core/src/exec.rs`（`ExecFlags`: 各ビットの意味、文字コードの優先、前後の情報の決まり、`OUT_AUTO`）
  - `crates/matsu-commands/src/sakura/macro_functions.rs`（`ExecCommand` の行。キーマクロに記録される）
- 試験: `crates/matsu-ui/src/exec/tests.rs` の `the_extra_information_can_be_left_out`、`the_encodings_of_the_input_and_the_output_are_chosen_by_their_bits`、`the_folder_is_used_only_when_asked_for`、`an_empty_command_is_told_and_a_missing_one_stops_the_macro`。`crates/matsu-core/src/exec/tests.rs` の `the_extra_information_follows_the_destination_and_the_two_bits`
