# 自動判別→SJISコード変換

選んでいる文字列の文字コードを自動で見分けて、本来の文字に直します。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「文字コード変換」の「自動判別→SJISコード変換」 |
| 機能名 | `F_CODECNV_AUTO2SJIS` |
| マクロ | `AutoToSJIS` |
| 既定のキー | `Alt+F12` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[文字コード変換の考え方](code-basics.md)を見てください。

## 動き

選んでいる文字列を Shift_JIS のバイト列に戻し、そのバイト列の文字コードを見分けて、その文字コードとして読み直します。見分ける対象は、次のものです。

- ISO-2022-JP（メールの JIS）
- EUC-JP
- UTF-16（リトルエンディアンとビッグエンディアン）
- UTF-8
- UTF-7

見分けた結果がこれ以外（たとえば Shift_JIS そのもの）のときは、変換できません。文字列は変わらず、警告音が鳴ります。

見分けの既定の文字コードは UTF-8 です。ISO-2022-JP と見分けたときは、メールの見出しの `=?ISO-2022-JP?B?...?=`（B と Q）も読みます。

箱型の選択のときは、行ごとに変えます。変換できない行があっても、できる行は変わります。

## サクラエディタとの違い

- 見分けは、`uchardet` ではなく MATSU の見分け（`matsu_core::encoding::detect`）です。結果が違う文字列があります。
- 既定の文字コードは、タイプ別設定の値ではなく UTF-8 に決めています（コマンドからタイプ別設定がまだ見えないため）。
- 変換できないとき、サクラは「変換でエラーが発生しました」を出して選択を同じ文字列で置き換えます。MATSU は警告音だけで、文書を変えません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_CODECNV_AUTO2SJIS`、`sakura_core/convert/CConvert_CodeAutoToSjis.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/codecnv_auto2_sjis.rs`、`crates/matsu-commands/src/convert/code.rs` の `auto_to_sjis`
- 試験: `crates/matsu-commands/src/convert/code.rs` の `sjis_to_other_codes_shows_the_bytes_as_sjis`、`the_two_directions_undo_each_other`、`crates/matsu-commands/src/commands/convert/codecnv_auto2_sjis.rs` の試験
