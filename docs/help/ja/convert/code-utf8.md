# UTF-8→SJISコード変換

UTF-8 のバイト列として読み直します。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「文字コード変換」の「UTF-8→SJISコード変換」 |
| 機能名 | `F_CODECNV_UTF82SJIS` |
| マクロ | `UTF8toSJIS` |
| 既定のキー | `Alt+F8` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[文字コード変換の考え方](code-basics.md)を見てください。

## 動き

選んでいる文字列を、UTF-8のバイト列を Shift_JIS として読んだときに出る文字列だとみなし、本来の文字に直します。

1. 選んでいる文字列を、Shift_JIS のバイト列に戻します（Shift_JIS にない文字は `?` になります）
2. そのバイト列を、UTF-8として読み直します
3. 読み直した文字列で、選択を置き換えます

UTF-8として読めないバイトは、代わりの文字で保たれます。そのため、[逆向きの変換](code-basics.md#逆向きの変換)で元のバイトに戻せます。

## 例

UTF-8 のファイルを Shift_JIS として開くと、日本語が `譌･譛ｬ隱` のような文字になります。その文字列を選んでこの機能を実行すると、`日本語` に戻ります。

## サクラエディタとの違い

- 動きの違いは、共通の動きの節に書いたものだけです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_CODECNV_UTF82SJIS`、`sakura_core/convert/CConvert_CodeToSjis.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/codecnv_utf82_sjis.rs`、`crates/matsu-commands/src/convert/code.rs` の `to_sjis`
- 試験: `crates/matsu-commands/src/convert/code.rs` の `sjis_to_other_codes_shows_the_bytes_as_sjis`、`the_two_directions_undo_each_other`、`crates/matsu-commands/src/commands/convert/codecnv_utf82_sjis.rs` の試験
