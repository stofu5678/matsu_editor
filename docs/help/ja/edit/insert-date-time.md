# 日付挿入と時刻挿入

今の日付、または今の時刻を、カーソルの位置に入れます。選択があるときは、選択の範囲の代わりに入ります。

| 機能名 | メニュー | キー | マクロ | 入るもの |
|---|---|---|---|---|
| `F_INS_DATE` | 「編集」→「挿入」→「日付挿入」 | `Alt+;` | `InsertDate()` | 今日の日付 |
| `F_INS_TIME` | 「編集」→「挿入」→「時刻挿入」 | `Alt+:` | `InsertTime()` | 今の時刻 |

macOS では、`Alt` は `Option` キーです。`Option` を押しながら打つ文字との重なりについては、ADR-0008 を見てください。

## 書き方（書式）

書式は、共通設定「書式」の「日付書式」「時刻書式」で決まります。「標準」のときは、次の書式を使います。この書式は、サクラエディタの書式の初期値と同じです。

| 機能 | 標準の書式 | 出力の例 |
|---|---|---|
| 日付 | `yyyy'年'M'月'd'日('dddd')'` | `2026年10月15日(木曜日)` |
| 時刻 | `tthh'時'mm'分'ss'秒'` | `午前09時05分07秒` |

「カスタム」にすると、そこに書いた書式を使います。

## サクラエディタとの違い

- サクラエディタの「標準」は、Windows の地域の設定にある日付・時刻の形です。MATSU Editor には、その地域の設定がないので、「標準」は上の書式で代わりにします。
- 矩形選択のときの動きは、まだ作っていません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Insert.cpp` の `Command_INS_DATE`、`Command_INS_TIME`
- MATSU のコード: `crates/matsu-commands/src/commands/insert/insert_date.rs`、`insert_time.rs`（先頭のコメント）、設定の読み取りは `crates/matsu-commands/src/format_options.rs`
- 試験: 上の 2 つのファイルの `tests` モジュール
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`Alt+;` と `Alt+:` の行）
- 設定ページ: `crates/matsu-ui/src/common_settings/pages/format.rs`
