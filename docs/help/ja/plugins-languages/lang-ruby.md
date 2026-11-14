# Ruby

拡張子は `.rb` です。利用者が入れた Ruby で動かします。mruby の `.mruby` とは別です（[mruby](lang-mruby.md)）。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `ruby`（`PATH` から探します） |
| 版 | 下限を確かめません |
| プラグイン | 動かせません（動かせるのは QuickJS、Node.js、Python です） |

## 動き方

- `ruby -E UTF-8` で起動します。OS の言語設定にかかわらず、文字列とファイルは UTF-8 として扱います。
- スクリプトは、`load` で読み込みます。行番号は、ファイルのものです。
- `Editor` と `EditorError` は、トップレベルの定数としてあります。`require` は要りません。
- `Editor` の関数の括弧は、省けます（`Editor.InsText "文字"`）。

## 標準ライブラリ

利用者の Ruby の標準ライブラリと、入れてある gem が、そのまま使えます。`json` は、MATSU Editor が読み込みます。

## 文字コード

- 標準入出力は、バイトのまま扱います。出力は UTF-8 です。Windows でも、改行が CR LF に変わることはありません。
- スクリプトは UTF-8 で保存します。

## エラー

`EditorError` は `StandardError` を継承します。`code` を持ちます。

受けなかった例外は、Ruby の形で、標準エラーに出ます。スクリプトは、終了コード 1 で終わります。

## 根拠

- 起動の引数: `crates/matsu-plugin/src/session.rs` の `start`、`Language::default_interpreter`
- `load`、定数、出力: `crates/matsu-plugin/src/boot/boot.rb`
- 試験: `crates/matsu-plugin/src/tests.rs`
