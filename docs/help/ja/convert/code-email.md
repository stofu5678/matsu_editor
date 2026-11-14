# E-Mail(JIS→SJIS)コード変換

ISO-2022-JP（メールの JIS）のバイト列として読み直します。メールの見出しも読み解きます。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「文字コード変換」の「E-Mail(JIS→SJIS)コード変換」 |
| 機能名 | `F_CODECNV_EMAIL` |
| マクロ | `JIStoSJIS` |
| 既定のキー | `Alt+F11` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[文字コード変換の考え方](code-basics.md)を見てください。

## 動き

選んでいる文字列を、ISO-2022-JPのバイト列を Shift_JIS として読んだときに出る文字列だとみなし、本来の文字に直します。メールの見出しの `=?ISO-2022-JP?B?...?=`（Base64 の B と、Q）は、読み直す前に文字に直します。

1. 選んでいる文字列を、Shift_JIS のバイト列に戻します（Shift_JIS にない文字は `?` になります）
2. そのバイト列を、ISO-2022-JPとして読み直します
3. 読み直した文字列で、選択を置き換えます

ISO-2022-JPとして読めないバイトは、代わりの文字で保たれます。逆向きの変換で同じバイト列に戻るとは限りません（[逆向きの変換](code-basics.md#逆向きの変換)）。

## サクラエディタとの違い

- 見出しを読み解く処理は、サクラの `CodeBase` の MIME 見出しの処理を、MATSU の `matsu_core::encoding::mime` に書き直したものです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_CODECNV_EMAIL`、`sakura_core/convert/CConvert_CodeToSjis.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/codecnv_email.rs`、`crates/matsu-commands/src/convert/code.rs` の `to_sjis`
- 試験: `crates/matsu-commands/src/convert/code.rs` の `sjis_to_other_codes_shows_the_bytes_as_sjis`、`the_two_directions_undo_each_other`、`crates/matsu-commands/src/commands/convert/codecnv_email.rs` の試験
