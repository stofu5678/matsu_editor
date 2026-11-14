# SJIS→UTF-8コード変換

選んでいる文字列を、UTF-8 のバイト列にして、Shift_JIS の文字として見せます。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「文字コード変換」の「SJIS→UTF-8コード変換」 |
| 機能名 | `F_CODECNV_SJIS2UTF8` |
| マクロ | `SJIStoUTF8` |
| 既定のキー | `Alt+Shift+F8` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[文字コード変換の考え方](code-basics.md)を見てください。

## 動き

選んでいる文字列を、UTF-8のバイト列にして、そのバイト列を Shift_JIS の文字として見せます。[「UTF-8→SJIS」の変換](code-basics.md#逆向きの変換)の逆向きです。

1. 選んでいる文字列を、UTF-8のバイト列に書きます
2. そのバイト列を、Shift_JIS として読みます
3. 読んだ文字列で、選択を置き換えます

UTF-8にない文字は、UTF-8の書き方で代わりの形になります。

## サクラエディタとの違い

- 動きの違いは、共通の動きの節に書いたものだけです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_CODECNV_SJIS2UTF8`、`sakura_core/convert/CConvert_CodeFromSjis.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/codecnv_sjis2_utf8.rs`、`crates/matsu-commands/src/convert/code.rs` の `from_sjis`
- 試験: `crates/matsu-commands/src/convert/code.rs` の `sjis_to_other_codes_shows_the_bytes_as_sjis`、`the_two_directions_undo_each_other`、`crates/matsu-commands/src/commands/convert/codecnv_sjis2_utf8.rs` の試験
