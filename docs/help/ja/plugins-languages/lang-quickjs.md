# JavaScript（QuickJS）

拡張子は `.js` です。MATSU Editor に同梱の `matsu-script` が、QuickJS で動かします。利用者が入れるものは、ありません。Node.js で動かしたいときは、拡張子を `.mjs` か `.cjs` にします（[JavaScript（Node.js）と TypeScript](lang-node.md)）。

マクロと、プラグインのハンドラの、どちらも動かせる唯一の言語です。

## 処理系

| 項目 | 内容 |
|---|---|
| 処理系 | QuickJS-NG。Rust のクレート `rquickjs`（0.14）が、C のソースを取り込んで、一緒にビルドしたもの |
| ライセンス | MIT |
| 言語の範囲 | QuickJS が持つ範囲。ブラウザーや Node.js の機能は、ありません |

## 使えるもの

- 言語の組み込みのオブジェクト（`Promise`、`JSON`、`Date`、`RegExp`、`Map`、`Set` など）。
- `Editor`、`EditorError`、`console`。
- プラグインでは、さらに `Plugin`、`Outline`、`Indent`、`Complement`（[ハンドラの中で使えるもの](plugin-api.md)）。

次のものは、ありません。

- `require` と `module`。ファイルを分けて読み込む仕組みは、ありません。
- `import` 文と、最上位の `await`。スクリプトは、モジュールではなく、普通のスクリプトとして動くためです。
- Node.js の `fs`、`process` などの機能。

## マクロとして動かすとき

- スクリプトは、普通のスクリプトとして実行します。厳密モード（strict mode）です。宣言していない名前へ代入すると、エラーになります。
- `async` 関数や `.then` の処理は、スクリプトが終わったあと、すべて終わるまで待ちます。誰も受けなかった拒否（rejection）が残っていると、エラーで終わります。
- 再帰は、機械のスタックの 512 KB の手前で止まり、`RangeError` になります。

## `console`

| 呼び出し | 出し先 |
|---|---|
| `console.log`、`console.info`、`console.debug` | 標準出力 |
| `console.error`、`console.warn`、`console.trace` | 標準エラー |

値の出し方は、次のとおりです。文字列はそのまま、`Error` はスタックつき、オブジェクトは `JSON.stringify` の結果、関数は `[Function: 名前]` の形、`BigInt` は末尾に `n` を付けた形です。複数の引数は、空白で区切ります。出力はすぐに流れます。

## 値の受け渡し

- 整数の値の数（`2.0` など）は、整数として渡します。
- 引数の最後の `undefined` は、省略として扱います。配列やオブジェクトの中の `undefined` は、`null` になります。
- 関数、シンボル、`BigInt`、対になっていないサロゲートを含む文字列は、引数に渡せません。渡すと、`EditorError`（番号 `-32602`）になります。文字を置き換えて通すことは、しません。
- 入れ子が 64 段を超える値は、渡せません。自分自身を含む値（循環）が、スタックの溢れにならずに、ここで止まります。
- 2^53 を超える整数は、JavaScript の数として、下の桁が落ちます。

## エラー

`EditorError` は `Error` を継承して、`name` と `code` を持ちます。スタックは、呼んだ場所から始まります。

受けなかったエラーは、`スクリプト名:行:桁` つきのスタックとして、標準エラーに出ます。`throw` した値が `Error` でないときは、「Uncaught」に値を付けた形で出ます。どちらも、スクリプトは終了コード 1 で終わります。

## プラグインとして動かすとき

- ハンドラのファイルごとに、別の実行環境（QuickJS のランタイム）が作られます。プロセスが残っている間、同じファイルの次の呼び出しは、同じ実行環境を使います。
- ハンドラの中身は、関数の本体として実行します。厳密でないモード（sloppy モード）が既定です。ファイルの先頭に `"use strict"` と書くと、厳密になります。詳しくは、[ハンドラの中で使えるもの](plugin-api.md)にあります。
- 呼び出しの間に残したいものは、`globalThis` に置きます。

## 根拠

- 処理系: `crates/matsu-script/Cargo.toml`、`crates/matsu-script/src/quickjs.rs`、`docs/poc/json-rpc.md`（「同梱のスクリプトホスト」。QuickJS-NG の記述）
- 厳密モード: `quickjs.rs` の `named`、`run_handler`（「rquickjs evaluates strictly by default」と、プラグインで緩めること）
- `console`、`EditorError`、スタック: `crates/matsu-script/src/quickjs.js`
- 値の変換と上限: `quickjs.rs` の `js_to_json`、`json_to_js`、`MAX_DEPTH`、`MAX_STACK`
- Promise: `quickjs.rs` の `Vm::finish`
- ライセンス: `THIRD_PARTY_NOTICES.txt`
- 試験: `crates/matsu-script/tests/quickjs.rs`（`a_script_is_a_classic_script_not_a_module`、`promises_settle_after_the_script_and_an_unhandled_rejection_ends_the_macro`、`a_runaway_recursion_is_an_error_not_a_crash` ほか）、`crates/matsu-script/tests/plugin_host.rs`
