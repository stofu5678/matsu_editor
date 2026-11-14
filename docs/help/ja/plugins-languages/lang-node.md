# JavaScript（Node.js）と TypeScript

利用者が入れた Node.js で動かします。拡張子で、動かし方が決まります。

| 拡張子 | 動き |
|---|---|
| `.mjs` | Node.js のモジュール（ES modules）として動く |
| `.cjs` | Node.js の CommonJS として動く |
| `.ts` | Node.js が型を取り除いて動かす（TypeScript） |

`.js` は、Node.js ではなく、同梱の QuickJS で動きます（[JavaScript（QuickJS）](lang-quickjs.md)）。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `node`（`PATH` から探します） |
| 版 | `.mjs` と `.cjs` は、版の下限を確かめません。`.ts` は Node.js 22.18 以降（22 系）、23.6 以降（23 系）、または 24 以降 |
| 試した版 | Node.js v24、v25、22.23（根拠に挙げた記録と CI） |
| プラグイン | 動かせません（今は QuickJS だけです） |

`.ts` を、古い Node.js で動かそうとすると、「TypeScript のマクロには Node.js 22.18 以降が要ります (見つかったのは vX)」と出て、終わります。

型を取り除くのは、Node.js の機能です。どの書き方が使えるかは、利用者の Node.js の説明に従います。型は空白に置き換わるので、行番号は変わりません。

`.ts` は、小文字だけが TypeScript です。`.TS` は、TypeScript のマクロとして扱いません。

## 標準ライブラリ

利用者の Node.js の標準ライブラリが、そのまま使えます（`fs`、`path` など）。`Editor` と `EditorError` は、最初から、グローバルにあります。`require` や `import` は、書かなくて構いません。

`Editor` の関数の呼び出しは、答えが返るまで待ちます（同期）。`await` は要りません。待っている間、Node.js のイベントループは進みません。

## 文字コード

- 出力は UTF-8 です。
- スクリプトは UTF-8 で保存します。

## エラー

`EditorError` は、エラーのコードを `code` に持ちます。

受けなかったエラーは、Node.js の形で、スタックつきで標準エラーに出ます。スクリプトは、終了コード 1 で終わります。

## 根拠

- 起動の引数と名前: `crates/matsu-plugin/src/session.rs` の `Language::default_interpreter`、`start`（`--require`）、`node_strips_types`、`Language::of_file`
- 型の取り除きと版の確認: `crates/matsu-plugin/src/boot/boot.cjs`。`docs/adr/0025-script-languages.md`
- 同期の呼び出し、出力: `boot.cjs`（冒頭の説明）
- 試した版: `docs/poc/json-rpc.md`（「確かめたこと」）。`.github/workflows/ci.yml`（Node.js 22.23）
- 試験: `crates/matsu-plugin/src/tests.rs`、`crates/matsu-script/tests/quickjs.rs` の `node_extensions_stay_on_node`
