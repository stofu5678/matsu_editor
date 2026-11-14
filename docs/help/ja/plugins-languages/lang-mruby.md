# mruby

拡張子は `.mruby` です。MATSU Editor に同梱の `matsu-script` が、mruby で動かします。利用者が入れるものは、ありません。ビルドにも Ruby は要りません。

`.rb` は、利用者が入れた Ruby で動きます（[Ruby](lang-ruby.md)）。

## 処理系

| 項目 | 内容 |
|---|---|
| 版 | mruby 4.0.0（上流のタグ `4.0.0`） |
| 作り | C のソースと生成済みのファイルを、一緒にビルドしたもの（クレート `matsu-mruby-sys`） |
| ライセンス | MIT |
| プラグイン | 動かせません（今は QuickJS だけです） |

mruby は、組み込み向けの小さな Ruby です。CRuby と同じではありません。CRuby にある標準ライブラリの多くが、ありません。

## 標準ライブラリ

マクロが使う範囲に絞って、次のものを入れてあります。

| 種類 | 入れたもの |
|---|---|
| 言語の拡張 | `String`、`Array`、`Hash`、`Numeric`、`Range`、`Symbol`、`Object`、`Kernel`、`Class`、`Proc`、`Comparable`、`Enumerable` の拡張メソッド |
| 数 | 大きな整数（`2**70` など）、`Math`、`rand` と `Random` |
| 書式 | `format`、`sprintf`、`String#%`、`Array#pack`、`String#unpack` |
| データ | `Set`、`Struct`、`Data` |
| 時刻 | `Time` と `strftime` |
| 制御 | `Enumerator`（遅延と連結を含む）、`Fiber`、`catch` と `throw`、`loop` |
| 反射 | `define_method`、`instance_variable_get`、`Method`、`eval`、`binding`、`ObjectSpace` |

次のものは、入れていません。ファイルや外部とのやり取りは、すべてありません。

- `IO`、`File`、`Dir`、ソケット、`sleep`、外部コマンドの実行
- `Rational`、`Complex`

`require` は、ありません。

`puts`、`printf`、`pp`、`warn`、`$stdout`、`$stderr` は、mruby 本来の入出力のモジュールにあるので、MATSU Editor が定義し直しています。`puts` は、配列を平らにして 1 行ずつ出します。

## 文字コード

- スクリプトは UTF-8 で保存します。先頭の BOM は取り除きます。
- 文字列は UTF-8 です。UTF-8 として正しくない文字列を `Editor` の関数に渡すと、文字を置き換えずに、エラーになります。
- 整数の値の浮動小数点数は、整数として渡します。
- 引数の最後の `nil` は省略として扱います。括弧は省けます。

## エラー

`EditorError`（`StandardError` を継承）は、`code` を持ちます。呼んだ行は、例外の `backtrace` に出ます。`where` は、ありません。

受けなかった例外は、`名前:行: メッセージ (クラス名)` の形で、標準エラーに出ます。構文のエラーも、同じ形です。スクリプトは、終了コード 1 で終わります。

## 根拠

- 版と作り: `crates/matsu-mruby-sys/Cargo.toml`、`crates/matsu-mruby-sys/vendor/mruby/include/mruby/version.h`、`vendor/mruby/README.md`
- 入れた gem と、入れなかったもの: `crates/matsu-mruby-sys/vendor/mruby/README.md` の「入れた gem と理由」
- `puts` などの定義し直し、エラーの形: `crates/matsu-script/src/mruby_boot.rb`
- 引数と BOM: `crates/matsu-script/src/mruby.rs`
- ライセンス: `THIRD_PARTY_NOTICES.txt`
- 試験: `crates/matsu-script/tests/mruby.rs`
- 設計: `docs/poc/json-rpc.md`（「同梱のスクリプトホスト」）
