# コントロールコード入力

画面から打てない制御文字（ESC など）を、一覧から選んで入れます。

## 呼び出し方

- メニュー: 「編集」→「挿入」→「コントロールコード入力...」
- キー: 既定の割り当てはありません
- マクロ: `CtrlCodeDialog()`（ダイアログを開く）、`CtrlCode(コード)`（コードを直接入れる）
- 機能名: `F_CTRL_CODE_DIALOG`（ダイアログ）、`F_CTRL_CODE`（入力）

## ダイアログ

制御文字 33 個（0x00 から 0x1f と 0x7f）の一覧が出ます。列は次のとおりです。

| 列 | 例 |
|---|---|
| コード | `0x1B` |
| 表記 | `^[`（DEL は `^?`） |
| 名前 | `ESC` |
| 説明 | その文字の説明 |

- 最初は、先頭の行が選ばれています。
- 「OK」、`Enter`、行のダブルクリックで、その行の文字を入れます。`Esc` で取りやめます。
- キーで直接選ぶこともできます。英字 `A` から `Z` は 0x01 から 0x1a です。`@ [ \ ] ^ _ /` のキーは、順に NUL、ESC、FS、GS、RS、US、DEL です。US 配列のキーボードでは、`@` はバッククォートのキーの位置にあるので、そのキーも NUL を選びます。

選んだコードは、`CtrlCode(コード)` として入れます。そのため、キーマクロの記録にも残ります。

## `CtrlCode` の動き

- 引数は、コード（数）です。
- CR と LF も、文書の改行コードに直さず、そのままの文字を入れます。
- 入れたあとに、前の行の字下げ（インデント）が続くのは、Enter を打ったときと同じです（自動インデントが働いているとき）。

## まだできないこと

ダイアログのヘルプのボタンは、ありません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Insert.cpp` の `Command_CtrlCode_Dialog`、`sakura_core/cmd/CViewCommander.cpp` の `HandleCommand`（`F_CTRL_CODE`）
- MATSU のコード: `crates/matsu-ui/src/support/ctrl_code.rs`（ダイアログの仕様のコメント）、`crates/matsu-ui/src/ui_commands/support/ctrl_code_dialog.rs`、`crates/matsu-commands/src/commands/edit/ctrl_code.rs`
