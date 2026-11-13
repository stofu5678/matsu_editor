# 文字コード変換の考え方

「変換」メニューの「文字コード変換」にある機能は、ファイルの文字コードを変えるものではありません。**文字化けした文字列を、本来の文字に直す**ためのものです（この逆もできます）。

## 何が起きているか

たとえば UTF-8 のファイルを、Shift_JIS として開いたとします。バイト列は同じでも、読み方が違うので、`譌･譛ｬ隱` のような文字が並びます。この文字列は、Shift_JIS として読んだときの見え方です。

そこで、この文字列を選んで「UTF-8→SJISコード変換」を実行します。MATSU Editor は、次のことをします。

1. 選んでいる文字列を、Shift_JIS のバイト列に戻します。Shift_JIS にない文字は、`?` になります
2. そのバイト列を、UTF-8 として読み直します
3. 読み直した文字列で、選択を置き換えます

「xxx→SJIS」と名前の付いた機能は、このように「xxx のバイト列だったもの」を直します。名前に `SJIS→xxx` と付いた機能は、逆向きです。選んでいる文字列を xxx のバイト列に書き、そのバイト列を Shift_JIS の文字として見せます。

## 逆向きの変換 {#逆向きの変換}

「xxx→SJIS」と「SJIS→xxx」は、たがいに元へ戻せます。xxx として読めないバイトは、代わりの文字で保たれるので、往復しても同じバイト列に戻ります（サクラは、これを別の文字の範囲で保ちます）。

## 共通の動き

[変換の共通の動き](index.md#共通の動き)に加えて、次のことがあります。

- 変えるのは、選んでいる文字列だけです。文書の文字コードや BOM の設定は、変わりません
- 文書の文字コードを指定して開き直すには、ファイルのメニューの機能を使います
- 箱型の選択のときは、行ごとに変えます

## 機能の一覧

| メニュー | 向き | ページ |
|---|---|---|
| 自動判別→SJISコード変換 | 見分けて直す | [自動判別→SJISコード変換](code-auto.md) |
| E-Mail(JIS→SJIS)コード変換 | ISO-2022-JP を直す | [E-Mail(JIS→SJIS)コード変換](code-email.md) |
| EUC→SJISコード変換 | EUC-JP を直す | [EUC→SJISコード変換](code-euc.md) |
| UTF-16→SJISコード変換 | UTF-16 を直す | [UTF-16→SJISコード変換](code-utf16.md) |
| UTF-16BE→SJISコード変換 | UTF-16BE を直す | [UTF-16BE→SJISコード変換](code-utf16be.md) |
| UTF-8→SJISコード変換 | UTF-8 を直す | [UTF-8→SJISコード変換](code-utf8.md) |
| UTF-7→SJISコード変換 | UTF-7 を直す | [UTF-7→SJISコード変換](code-utf7.md) |
| SJIS→JISコード変換 | JIS に書く | [SJIS→JISコード変換](code-sjis-jis.md) |
| SJIS→EUCコード変換 | EUC-JP に書く | [SJIS→EUCコード変換](code-sjis-euc.md) |
| SJIS→UTF-8コード変換 | UTF-8 に書く | [SJIS→UTF-8コード変換](code-sjis-utf8.md) |
| SJIS→UTF-7コード変換 | UTF-7 に書く | [SJIS→UTF-7コード変換](code-sjis-utf7.md) |
| Base64デコードして保存 | ファイルへ書く | [Base64デコードして保存](base64-decode-save.md) |
| uudecodeして保存 | ファイルへ書く | [uudecodeして保存](uudecode-save.md) |

## 根拠

- サクラのソース: `sakura_core/convert/CConvert_CodeToSjis.cpp`、`CConvert_CodeFromSjis.cpp`、`CConvert_CodeAutoToSjis.cpp`
- MATSU のコード: `crates/matsu-commands/src/convert/code.rs`（先頭の説明）、`crates/matsu-core/src/encoding/`
- 試験: `crates/matsu-commands/src/convert/code.rs` の `utf8_read_as_sjis_is_fixed`、`the_two_directions_undo_each_other`
