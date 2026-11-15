# 挿入の関数

日付、時刻、制御コード、最近使ったファイルとフォルダーを、文書に入れる関数です。マクロから文字列を渡して入れる `InsText` などは、[クリップボード](clipboard.md) にあります。

## 表

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `InsertDate` | なし | なし | 今日の日付を、選択の代わりに、キャレットの位置へ入れる。書式は、共通設定の日付書式の種類で決まる。 | する | 種類 0 は、サクラの既定の書式 `yyyy'年'M'月'd'日('dddd')'`（例: `2026年10月15日(木曜日)`）を使う。サクラの種類 0 は OS の地域の書式で、MATSU Editor にはその仕組みがない。種類 1 は、共通設定に書いた書式を使う。矩形選択があるときは、日付の文を、矩形の各行の矩形の手前に入れる。矩形は、入れた文の幅だけ右へ動き、選択されたまま残る。 |
| `InsertTime` | なし | なし | 現在の時刻を、選択の代わりに、キャレットの位置へ入れる。書式は、共通設定の時刻書式の種類で決まる。 | する | 種類 0 は、サクラの既定の書式 `tthh'時'mm'分'ss'秒'`（例: `午前09時05分07秒`）を使う。種類 1 は、共通設定に書いた書式を使う。矩形選択があるときは、日付の文を、矩形の各行の矩形の手前に入れる。矩形は、入れた文の幅だけ右へ動き、選択されたまま残る。 |
| `CtrlCodeDialog` | なし | なし | 制御コード入力のダイアログを開く。選んだコードは、`CtrlCode` で入力される。 | しない | 記録なし |
| `CtrlCode` | 整数（文字コード。Unicode のコードポイント） | なし | 制御コード 1 文字を入力する。文字入力と同じだが、CR と LF は、文書の改行に直さず、そのまま入れる。入れた後に、行頭のインデントを、Enter のときと同じように引き継ぐ（自動インデント）。コードが文字にならない値のときは、エラーになる。 | する | 記録なし |
| `InsertFileUsedRecently` | なし | なし | 最近使ったファイルのパスを、新しい順に、1 行に 1 つずつ、キャレットの位置へ入れる。各行の後ろに文書の改行が付く。 | する | 履歴はアプリ全体のものなので、ウィンドウのコマンドとして動く。 |
| `InsertFolderUsedRecently` | なし | なし | 最近使ったフォルダーのパスを、新しい順に、1 行に 1 つずつ、キャレットの位置へ入れる。各行の後ろに文書の改行が付く。 | する | 同上 |

## 根拠

- 表の元: `crates/matsu-commands/src/sakura/macro_functions.rs`（`InsertDate` から `InsertFolderUsedRecently`）
- サクラ: `sakura_core/cmd/CViewCommander_Insert.cpp`（`Command_INS_DATE`、`Command_INS_TIME`、`Command_CtrlCode_Dialog`、`Command_INS_FILE_USED_RECENTLY`、`Command_INS_FOLDER_USED_RECENTLY`）、`sakura_core/cmd/CViewCommander.cpp`（`HandleCommand` の `F_CTRL_CODE`）
- MATSU のコード（`crates/matsu-commands/src/commands/`）: `insert/insert_date.rs`、`insert/insert_time.rs`、`edit/ctrl_code.rs`
- MATSU のコード（`crates/matsu-ui/src/`）: `ui_commands/support/ctrl_code_dialog.rs`、`ui_commands/history/ins_file_used_recently.rs`、`ins_folder_used_recently.rs`
- 試験: `insert/insert_date.rs`、`insert/insert_time.rs`、`edit/ctrl_code.rs` の各 `tests`
