# PHP

拡張子は `.php` です。利用者が入れた PHP（`php` のコマンドライン版）で動かします。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `php`（`PATH` から探します） |
| 版 | 下限を確かめません |
| プラグイン | 動かせません（今は QuickJS だけです） |

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

## 根拠

- 起動の引数: `crates/matsu-plugin/src/session.rs` の `start`
- `require`、`EditorError`、`$argv`: `crates/matsu-plugin/src/boot/boot.php`
- 試験: `crates/matsu-plugin/src/tests.rs`
