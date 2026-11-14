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
| プラグイン | 動かせます（`.mjs`、`.cjs`、`.ts`）。下の「プラグインとして動かすとき」 |

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

## プラグインとして動かすとき

ハンドラの拡張子が `.mjs`、`.cjs`、`.ts` のとき、利用者の Node.js が動かします。1 つのプラグインの、この 3 つの拡張子のハンドラは、同じ Node.js のプロセスで動きます。`.js` は QuickJS です。

- `Editor`、`EditorError`、`console` のほか、`Plugin`、`Outline`、`Complement`、`Indent` が、グローバルにあります（[ハンドラの中で使えるもの](plugin-api.md)）。`import` や `require` は、書かなくて構いません。
- 呼び出しのたびに、ファイルの最上位が最初から動きます。
  - `.cjs` は、同意したときに読んだ文面を、新しい CommonJS のモジュールとして動かします。
  - `.mjs` と `.ts` は、そのファイルを、呼び出しのたびに新しい名前で読み込みます。最上位の `await` が使えます。隣のファイルの `import` もできます。
- 次の呼び出しまで残したいものは、`globalThis` に置きます。プロセスが残っている（`UseCache=1` か `Resident=1`）間だけ、残ります。ハンドラのファイルごとの別の実行環境は、ありません。別のファイルの `globalThis` の値も見えます。
- 誰も受けなかった Promise の拒否が残ると、その呼び出しは失敗になります。
- `.ts` に Node.js 22.18 より古い版を使うと、その呼び出しは「TypeScript のプラグインには Node.js 22.18 以降が要ります」で失敗します。
- 呼び出しの間、Node.js のイベントループは、`await` のところ以外では進みません。`Editor` などの呼び出しは、マクロと同じく、答えが返るまで待ちます。

### 気をつけること

- `.mjs` と `.ts` は、呼び出しのたびに新しいモジュールとして読み込み、Node.js はそれを捨てません。常駐するプラグインで、何度も呼ばれるもの（`SmartIndent`、入力補完）に使うと、呼ぶたびにメモリが少しずつ増えます。何度も呼ばれるジャックには、`.cjs` を使ってください。
- `.mjs` と `.ts` は、同意を確かめたあとで、Node.js がファイルを読み直します。確かめた文面と同じファイルを読みますが、確かめてから読むまでの間にファイルを書き換えられると、書き換えたものが動きます。`.cjs` は、確かめた文面をそのまま動かします。
- `.mjs` と `.ts` は、読み込んだファイルの行番号で、エラーが出ます。

- 起動の引数と名前: `crates/matsu-plugin/src/session.rs` の `Language::default_interpreter`、`start`（`--require`）、`node_strips_types`、`Language::of_file`
- 型の取り除きと版の確認: `crates/matsu-plugin/src/boot/boot.cjs`。`docs/adr/0025-script-languages.md`
- 同期の呼び出し、出力: `boot.cjs`（冒頭の説明）
- 試した版: `docs/poc/json-rpc.md`（「確かめたこと」）。`.github/workflows/ci.yml`（Node.js 22.23）
- プラグインの待ち受け: `boot.cjs` の `servePlugin`。試験は `crates/matsu-plugin/src/external_plugin_tests.rs`、サンプルは `crates/matsu-plugin/samples/plugins/HelloNode`
- 試験: `crates/matsu-plugin/src/tests.rs`、`crates/matsu-script/tests/quickjs.rs` の `node_extensions_stay_on_node`
