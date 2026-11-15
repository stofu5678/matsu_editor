# Perl

拡張子は `.pl` です。利用者が入れた Perl で動かします。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `perl`（`PATH` から探します） |
| 版 | JSON を扱うために、Perl 5.14 から標準に入った `JSON::PP` を使います。それより古い Perl は、確かめていません |
| プラグイン | 動かせます。下の「プラグインとして動かすとき」 |

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

## プラグインとして動かすとき

ハンドラの拡張子が `.pl` のとき、利用者の Perl が動かします。

- ハンドラの文面は、MATSU Editor が呼び出しのたびに渡します（同意のときに確かめた文面です）。`eval` で、`package main` として動かします。`#line` の指定つきなので、行番号は、ハンドラのファイルのものです。
- `Editor`、`EditorError`、`Plugin`、`Outline`、`Complement`、`Indent` は、クラスです。クラスメソッドで呼びます（`Plugin->GetOption('節', 'キー')`。ジャックの名前は `Plugin->jack`、引数は `Plugin->args`、プラグインの ID は `Plugin->id`、フォルダーは `Plugin->dir`）。`Indent->GetChar` は、`SmartIndent` のときだけ、打った文字を返します（ほかの場面では空の文字列です）（[ハンドラの中で使えるもの](plugin-api.md)）。`use` は要りません。
- ハンドラは、MATSU Editor の側の `my` 変数を見ません（`@_` も空です）。ハンドラの `my` 変数は、その呼び出しの中だけのものです。プロセスが残っている（`UseCache=1` か `Resident=1`）間、次の呼び出しに引き継ぎたい値は、`our $count; $count++;` のような、パッケージ変数に持たせます。`sub` で定義した関数も、残ります（次の呼び出しで定義し直しても、エラーになりません）。
- `return` は、そのハンドラを終えるだけです。`exit` も、そのハンドラを終えるだけで、失敗ではありません（`exit` を、ハンドラだけを終える形に置き換えています。ハンドラ自身の `eval` で包んだ `exit` は、その `eval` に捕まります）。`die` は、その呼び出しを失敗にします。なお、ハンドラを終えるときの `return` に値がなく、その前にハンドラ自身の `eval { ... }` が失敗して `$@` が残っていると、失敗として扱われます。`return 1;` のように値を付けるか、`$@ = '';` で消してください。
- `require 'lib/word.pl';` のように、プラグインのフォルダーの中のファイルを読めます（`@INC` の先頭が、プラグインのフォルダーです）。読むファイルは、エディターがフォルダーを確かめた直後に、ディスクから読みます（Python、Ruby、PHP が import するファイルを読むのと同じです）。`require` したファイルは、一度読むと、読み直しません。
- 失敗のメッセージは、Perl のふつうの形です（`... at ハンドラ名 line 行.`）。構文の誤り（`syntax error`）、`Editor` の失敗（`EditorError 番号: 理由`）も、同じです。プロセスは残り、次の呼び出しを受けます。
- `Plugin->GetOption('節', 'キー')` などの、オブジェクトの関数は、同じ綴りの関数です。引数の最後の `undef` は、省略として扱います。

## 根拠

- 起動の引数: `crates/matsu-plugin/src/session.rs` の `Language::Perl`、`start`（`PERL5OPT`）
- プラグインの待ち受け: `crates/matsu-plugin/src/boot/boot.pl` の `serve`、`describe`、`crates/matsu-plugin/src/boot/run_user.pl` の `run_handler`。試験は `crates/matsu-plugin/src/external_plugin_tests/powershell_perl.rs`、サンプルは `crates/matsu-plugin/samples/plugins/HelloPerl`
- 標準入力から渡すこと、`JSON::PP`、`#line`、文字コード、`EditorError`: `crates/matsu-plugin/src/boot/boot.pl`、`run_user.pl`
- 試験: `crates/matsu-plugin/src/tests.rs`
