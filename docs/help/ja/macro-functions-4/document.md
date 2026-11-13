# マクロの関数（4）: 文書と選択の状態

文書のファイル名、行の文字列、選択の位置、入力のモード、改行と文字コード、Undo の可否、行の属性を返す関数の表です。共通の決まりは[この章の最初](index.md)にあります。

- 「キーマクロに記録」は、どの行も「しない」です。
- 位置を数えるときの用語を先に決めます。
  - 行: 改行から改行までの 1 行を「文書の行」と呼びます。折り返しで分かれた画面の 1 段は「折り返しの行」と呼びます。
  - 桁: 全角の文字は 2、タブはタブ位置までの幅で数える桁を「表示桁」と呼びます。UTF-16 の単位で数える位置を「文字の位置」と呼びます（サロゲートペアの文字は 2 と数える）。
  - どちらも 1 から数えます。

## ファイル名

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `GetFilename` | なし | 文字列 | 文書のファイルのフルパス。ファイルのない文書では空の文字列 | しない | 違いなし |
| `GetSaveFilename` | なし | 文字列 | 保存で書き込むファイルのフルパス。ファイルのない文書では空の文字列 | しない | 通常は `GetFilename` と同じ。保存前に動く登録マクロ（共通設定の「マクロ」の「保存前」）の中では、いま保存しようとしているファイルのパスになる。サクラエディタは、ショートカット（`.lnk`）から開いたときだけ違う。MATSU Editor にはその仕組みがない |

## 行の文字列と行数

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `GetLineCount` | 1 予約（数。0 を渡す） | 数 | 文書の行の数。最後の改行のあとの空の位置は、行に数えない。空の文書は 0 | しない | 0 以外を渡すとエラー |
| `GetLineStr` | 1 行番号（数） | 文字列 | 文書の行の文字列。行末に改行があれば、改行を付ける。行番号は 1 から。0 はキャレットのある行。最後の行より後ろは空の文字列 | しない | 改行は、文書の改行が CR LF でも CR でも、必ず `\n` 1 文字で返る。負の数はエラー |
| `GetSelectedString` | 1 予約（数。読まない） | 文字列 | 選択している文字列。選択がなければ空の文字列。矩形選択は、矩形に入る各行の文字列（行の間は改行） | しない | 改行は、文書が持つ改行（CR LF、CR、LF）をすべて `\n` に直す |

引数が「予約」の関数は、MATSU Editor が値を読みません。ただし `GetLineCount` は、0 以外を渡すとエラーにします。

## 選択の位置

選択の「始まり」と「終わり」は、位置の小さい方が始まりです。どちらの向きで選んでも、始まりは先に出ます。どの関数も、選択がなければ 0 を返します。

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `IsTextSelected` | なし | 数 | 選択の種類。0 は選択なし、1 は通常の選択、2 は矩形選択 | しない | 違いなし。矩形選択は、何かを囲んでいるときに 2 |
| `GetSelectLineFrom` | なし | 数 | 選択の始まりの、折り返しの行の番号（1 から） | しない | 折り返しているときは、文書の行の番号とずれる |
| `GetSelectColmFrom` `GetSelectColumnFrom` | なし | 数 | 選択の始まりの表示桁（1 から） | しない | 2 つの名前は同じ関数。折り返した行の字下げも桁に入る |
| `GetSelectLineTo` | なし | 数 | 選択の終わりの、折り返しの行の番号 | しない | `GetSelectLineFrom` と同じ |
| `GetSelectColmTo` `GetSelectColumnTo` | なし | 数 | 選択の終わりの表示桁 | しない | 2 つの名前は同じ関数 |

- 矩形選択のときは、矩形の 2 つの角の位置を返します。角は、矩形としてではなく、位置として並べます（行が小さい方、行が同じなら桁が小さい方が「始まり」）。
- `GetSelectColmFrom` の綴りは、サクラエディタの古い名前です。新しい綴りの `GetSelectColumnFrom` と同じ値を返します。

## 入力のモード、改行、文字コード

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `IsInsMode` | なし | 数 | 1 は挿入モード、0 は上書きモード | しない | 違いなし |
| `GetCharCode` | なし | 数 | 文書の文字コード（番号は下の表） | しない | サクラの表にない文字コード（GBK など）は、`matsu.ini` で使うその文字コードの番号を返す。ウィンドウがないときは、エラー |
| `GetLineCode` | なし | 数 | Enter で入る改行の種類（番号は下の表） | しない | 3 から 5 は、サクラエディタにない値（拡張の改行）。文書の最初の改行が基になる |
| `IsTextSelectingLock` | なし | 数 | 選択ロック（移動で選択を広げる状態）の種類。0 は切れている、1 は入っている、2 は入っていて矩形選択 | しない | 違いなし |

`GetCharCode` の番号は、次のとおりです。

| 番号 | 文字コード |
|---|---|
| 0 | Shift_JIS |
| 1 | JIS |
| 2 | EUC-JP |
| 3 | UTF-16（リトルエンディアン） |
| 4 | UTF-8 |
| 5 | UTF-7 |
| 6 | UTF-16（ビッグエンディアン） |
| 7 | CESU-8 |
| 8 | Latin1 |

`GetLineCode` の番号は、次のとおりです。

| 番号 | 改行 |
|---|---|
| 0 | CR LF |
| 1 | CR |
| 2 | LF |
| 3 | NEL（U+0085） |
| 4 | LS（U+2028） |
| 5 | PS（U+2029） |

## Undo と行の属性

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `IsPossibleUndo` | なし | 数 | 元に戻せる操作があれば 1、なければ 0 | しない | 違いなし |
| `IsPossibleRedo` | なし | 数 | やり直せる操作があれば 1、なければ 0 | しない | 違いなし |
| `GetLineAttribute` | 1 行番号（数）、2 種類（数） | 数 | 文書の行の属性を返す。行番号は 1 から。0 はキャレットのある行。種類は下の表 | しない | 下の説明を見る |

`GetLineAttribute` の種類は、次のとおりです。

| 種類 | 返る値 |
|---|---|
| 0 | 最後の保存のあとに変えた行なら 1、そうでなければ 0 |
| 1 | その行を最後に変えた Undo の単位の通し番号 |
| 2 | ブックマークがあれば 1、なければ 0 |
| 3 | 差分の印。0 は印なし、1 は追加、2 は変更、3 は削除、4 は終わりより後ろの削除 |
| 4 | アウトラインの解析で、その行に項目が見つかっていれば 1、なければ 0 |

- 負の行番号、文書の行の数を超える行番号、0 から 4 以外の種類は、エラーです。最後の改行のあとの空の位置は、行ではないので、エラーです。
- 種類 3 と 4 は、差分表示とアウトラインを使ったあとの印を返します。使っていなければ 0 です。

## 根拠

- サクラのソース: `sakura_core/macro/CMacro.cpp` の `CMacro::HandleFunction`（`F_GETFILENAME`、`F_GETLINESTR`、`F_GETSELLINEFROM`、`F_ISTEXTSELECTED`、`F_GETLINEATTRIBUTE` など）
- MATSU のコード: `crates/matsu-plugin/src/api.rs`（`get_filename`、`get_line_count`、`get_line_str`、`get_selected_string`、`is_text_selected`）、`crates/matsu-plugin/src/functions/state.rs`（選択の位置、モード、改行、文字コード、Undo、`get_line_attribute`、`get_save_filename`）、`crates/matsu-plugin/src/bridge.rs`（`selected_text`、`selection_kind`）、`crates/matsu-commands/src/context/mod.rs` の `save_path`
- 試験: `crates/matsu-plugin/src/functions/state.rs` の `a_selection_is_counted_from_1_in_layout_units`、`a_box_selection_reports_its_corners`、`the_line_ending_and_the_encoding`、`line_attribute_gives_the_diff_mark_and_the_outline_mark`、`the_save_filename_is_the_file_of_the_save_for_the_macro_that_runs_before_it`、`crates/matsu-plugin/src/api.rs` の `get_line_str_includes_the_line_break`
