# PHP

拡張子は `.php` です。利用者が入れた PHP（`php` のコマンドライン版）で動かします。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `php`（`PATH` から探します） |
| 版 | 下限を確かめません |
| プラグイン | 動かせます。下の「プラグインとして動かすとき」 |

## 動き方

- スクリプトは、`require` で、グローバルの場所から読み込みます。行番号は、ファイルのものです。
- ファイルは、ふつうの PHP のファイルと同じく、`<?php` で始めます。
- `Editor` は、静的メソッドを持つクラスです（`Editor::InsText("文字");`）。`EditorError` は、`Exception` を継承するクラスです。`require` は要りません。
- スクリプトの `$argv` と `$_SERVER['SCRIPT_FILENAME']` は、スクリプト自身を指します。
- PHP は、次の設定で起動します。エラーが、標準エラーに、HTML でない平らな文として、1 回だけ出るようにするためです。

| 設定 | 値 |
|---|---|
| `display_errors` | `stderr` |
| `log_errors` | `0` |
| `html_errors` | `0` |
| `error_reporting` | `E_ALL` |
| `output_buffering` | `0` |

## 標準ライブラリ

利用者の PHP の標準関数と、有効な拡張が、そのまま使えます。

## 文字コード

- スクリプトは UTF-8 で保存します。
- PHP の文字列はバイト列です。`Editor` の関数に渡す文字列は、正しい UTF-8 にしてください。そうでないと、送るときに例外が起きます（`json_encode` が失敗するため）。

## エラー

`EditorError` の `getCode()` が、エラーの番号です。

受けなかった例外や、PHP のエラーは、行番号つきで標準エラーに出ます。終了コードは、PHP が決めます。

## プラグインとして動かすとき

ハンドラの拡張子が `.php` のとき、利用者の PHP が動かします。

- ハンドラの文面は、`eval` で、グローバルの場所で動きます。ファイルは、ふつうの PHP のファイルと同じく、`<?php` で始めます（`declare(strict_types=1);` も書けます）。行番号は、ファイルのものです。
- `Editor`、`EditorError`、`Plugin`、`Outline`、`Complement`、`Indent` は、クラスです。静的メソッドで呼びます（`Plugin::GetOption('節', 'キー')`。ジャックの名前は `Plugin::$jack`、引数は `Plugin::$args`）。`Indent::GetChar()` は、`SmartIndent` のときだけ、打った文字を返します（ほかの場面では空の文字列です）。`require` は要りません（[ハンドラの中で使えるもの](plugin-api.md)）。
- 変数は、グローバル変数です。プロセスが残っている（`UseCache=1` か `Resident=1`）間、次の呼び出しに引き継ぎます。ファイルごとには分けません。文面は呼び出しのたびに最初から動くので、前の呼び出しの値は、`$count = ($count ?? 0) + 1` のようにして読みます。
- ハンドラの中で `function` や `class` を宣言すると、2 回目の呼び出しで、PHP が「Cannot redeclare」で止まります（プロセスが終わり、失敗になります）。宣言は別のファイルに書いて、`require_once` してください。
- `require_once` などの探す場所（`include_path`）の先頭は、プラグインのフォルダーです。`require_once 'lib/word.php';` のように、フォルダーの中のファイルを読めます。
- `exit` と `die` は、PHP のプロセスを終わらせます。ハンドラの終わりには使わないでください（失敗になります）。
- 例外が出ると、その呼び出しは失敗になります。メッセージは、ハンドラのファイルの行番号つきです。構文の誤り（`ParseError`）も、同じです。PHP の警告は、標準エラーに出ます。その行番号は、`eval()'d code` の行です。
- `Plugin::GetOption('節', 'キー')` などの、オブジェクトの関数は、同じ綴りの関数です。引数の最後の `null` は、省略として扱います。

## 根拠

- 起動の引数: `crates/matsu-plugin/src/session.rs` の `start`
- `require`、`EditorError`、`$argv`: `crates/matsu-plugin/src/boot/boot.php`
- プラグインの待ち受け: `crates/matsu-plugin/src/boot/boot.php` の `MatsuBoot::nextInvoke`、`code`、`describe`。試験は `crates/matsu-plugin/src/external_plugin_tests/ruby_php.rs`、サンプルは `crates/matsu-plugin/samples/plugins/HelloPhp`
- 試験: `crates/matsu-plugin/src/tests.rs`
