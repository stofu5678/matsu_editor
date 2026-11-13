# 選択範囲内全行コピー

選択している行を、行全体に広げてコピーします。「高度な操作」の中の 3 つの操作が、写し方だけ違います。

| 機能名 | メニュー | キー | マクロ | 写し方 |
|---|---|---|---|---|
| `F_COPYLINES` | 選択範囲内全行コピー | `Ctrl+@`（英語配列ではバッククォート） | `CopyLines()` | そのまま |
| `F_COPYLINESASPASSAGE` | 選択範囲内全行引用符付きコピー | `Ctrl+.` | `CopyLinesAsPassage()` | 各行の先頭に引用符を付ける |
| `F_COPYLINESWITHLINENUMBER` | 選択範囲内全行行番号付きコピー | `Ctrl+:` | `CopyLinesWithLineNumber()` | 各行の先頭に行番号を付ける |

## 動き

- 選択の始まりの画面の行の先頭から、終わりの画面の行の終わりまでを、行全体としてコピーします。折り返し表示では、折り返した 1 行が単位です。
- 選択が、終わりの行の先頭で終わっているときは、その行は含めません（`ab` の途中から、次の行 `cd` の先頭までを選んだときは、`ab` の行だけを写します）。
- 選択がないときは、何もしません。
- 引用符は、共通設定「書式」の「引用符」で決めます。初期値は `> ` です。
- 行番号は ` 12:` のように付きます。

## サクラエディタとの違い

行番号付きのコピーで、サクラエディタは、折り返しで分かれた画面の行ごとに番号を付けます。MATSU Editor は、文書の行（改行で区切った行）に番号を付けます。折り返しのある文章だけで違いが出ます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Clipboard.cpp` の `Command_COPYLINES`、`Command_COPYLINESASPASSAGE`、`Command_COPYLINESWITHLINENUMBER`、`sakura_core/view/CEditView.cpp` の `CopySelectedAllLines`
- MATSU のコード: `crates/matsu-commands/src/commands/clipboard/copy_lines.rs`、`copy_lines_as_passage.rs`、`copy_lines_with_line_number.rs`
- 試験: `crates/matsu-commands/src/commands/clipboard/tests.rs` の `copy_lines_widens_the_selection_to_whole_rows`
- 設定の項目名: `crates/matsu-ui/src/common_settings/pages/format.rs`（`szInyouKigou`）
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`
