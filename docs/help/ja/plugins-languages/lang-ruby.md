# Ruby

拡張子は `.rb` です。利用者が入れた Ruby で動かします。mruby の `.mruby` とは別です（[mruby](lang-mruby.md)）。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `ruby`（`PATH` から探します） |
| 版 | 下限を確かめません |
| プラグイン | 動かせます。下の「プラグインとして動かすとき」 |

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

## プラグインとして動かすとき

ハンドラの拡張子が `.rb` のとき、利用者の Ruby が動かします。

- ハンドラの文面は、ファイルごとに 1 つずつある `Object` の中で、`instance_eval` で動きます。`self` がそのオブジェクトなので、`@変数` と、ハンドラで定義した `def` は、プロセスが残っている（`UseCache=1` か `Resident=1`）間、同じファイルの次の呼び出しに引き継ぎます。文面は呼び出しのたびに最初から動くので、前の呼び出しの値は、`@count = (@count || 0) + 1` のようにして読みます。ローカル変数は、引き継ぎません。別のファイルのオブジェクトは、見えません。
- `Editor`、`EditorError`、`Plugin`、`Outline`、`Complement`（と、`SmartIndent` のとき `Indent`）は、トップレベルの定数としてあります。`require` は要りません（[ハンドラの中で使えるもの](plugin-api.md)）。
- ハンドラの中で定数を定義すると、2 回目の呼び出しで「already initialized constant」の警告が出ます。定数は、別のファイルに書いて `require` してください。
- `require` の探す場所の先頭は、プラグインのフォルダーです。`require 'lib/word'` のように、フォルダーの中のファイルを読めます。`require` したファイルは、一度読むと、読み直しません。
- `exit` は、そのハンドラを終えるだけで、失敗ではありません。`return` も、そのハンドラを終えるだけです（途中で終えるのに使えます）。ハンドラのローカル変数は、MATSU Editor の側の変数とは別で、書き換えても影響しません。
- 例外が出ると、その呼び出しは失敗になります。メッセージは、ハンドラのファイルの行番号つきです。
- `Plugin.GetOption('節', 'キー')` などの、オブジェクトの関数は、同じ綴りの関数です。引数の最後の `nil` は、省略として扱います。

## 根拠

- 起動の引数: `crates/matsu-plugin/src/session.rs` の `start`、`Language::default_interpreter`
- `load`、定数、出力: `crates/matsu-plugin/src/boot/boot.rb`
- プラグインの待ち受け: `crates/matsu-plugin/src/boot/boot.rb` の `MatsuPlugin.serve`。試験は `crates/matsu-plugin/src/external_plugin_tests/ruby_php.rs`、サンプルは `crates/matsu-plugin/samples/plugins/HelloRuby`
- 試験: `crates/matsu-plugin/src/tests.rs`
