# Perl

拡張子は `.pl` です。利用者が入れた Perl で動かします。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `perl`（`PATH` から探します） |
| 版 | JSON を扱うために、Perl 5.14 から標準に入った `JSON::PP` を使います。それより古い Perl は、確かめていません |
| プラグイン | 動かせません（今は QuickJS だけです） |

## 動き方

- Perl は、`perl -` で起動します。MATSU Editor が、起動用のプログラムと `Editor` の定義を、標準入力から渡します。ファイルを開く方法を使わないので、Windows の Perl が、ファイル名を ANSI コードページで開いて失敗することが、ありません。
- 環境変数 `PERL5OPT` は、外して起動します。モジュールや設定を勝手に切り替えられないようにするためです。
- スクリプトは、`#line` の指定つきで `eval` します。エラーには、スクリプト自身の名前と行が出ます。
- スクリプトに `use strict` や `use warnings` は付きません。`perl script.pl` で動かしたときと同じです。必要なら、自分で書きます。
- `Editor` は、クラスメソッドで呼びます（`Editor->InsText("文字");`）。`use` は要りません。
- `$0` は、スクリプトの名前です。

## 標準ライブラリ

利用者の Perl の標準モジュールと、入れてあるモジュールが、そのまま使えます。

## 文字コード

- スクリプトの文字列リテラルは、文字（decode した文字列）として扱います。
- 標準出力と標準エラーには、`:raw:utf8` の層を付けます。文字列は、文字のまま UTF-8 で書き出します。Windows でも、改行は CR LF に変わりません。
- スクリプトは UTF-8 で保存します（BOM は取り除きます）。

## エラー

`Editor` の関数が失敗すると、`EditorError` のオブジェクトを `die` で投げます。

| メソッド | 内容 |
|---|---|
| `code` | エラーの番号 |
| `message` | 理由 |
| `file`、`line` | 呼んだ場所 |
| `as_string` | `EditorError 番号: 理由 at ファイル line 行.` |

Perl のふつうのエラーと同じ、`... at ファイル line 行.` の形で、標準エラーに出ます。スクリプトは、終了コード 1 で終わります。

## 根拠

- 起動の引数: `crates/matsu-plugin/src/session.rs` の `Language::Perl`、`start`（`PERL5OPT`）
- 標準入力から渡すこと、`JSON::PP`、`#line`、文字コード、`EditorError`: `crates/matsu-plugin/src/boot/boot.pl`、`run_user.pl`
- 試験: `crates/matsu-plugin/src/tests.rs`
