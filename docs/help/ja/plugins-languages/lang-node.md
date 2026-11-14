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
| プラグイン | `.cjs`、`.mjs`、`.ts` を動かせます。`.mjs` と `.ts` は、Node.js 22.15 以降（22 系）、23.5 以降（23 系）、または 24 以降。`.ts` は、そのうえで上の版。下の「プラグインとして動かすとき」 |

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

ハンドラの拡張子が `.cjs`、`.mjs`、`.ts` のとき、利用者の Node.js が動かします。`.js` は QuickJS です。

- `Editor`、`EditorError`、`console` のほか、`Plugin`、`Outline`、`Complement`、`Indent` が、グローバルにあります（[ハンドラの中で使えるもの](plugin-api.md)）。`Editor` などの書き足しは要りません。
- 次の呼び出しまで残したいものは、`globalThis` に置きます。プロセスが残っている（`UseCache=1` か `Resident=1`）間だけ、残ります。ハンドラのファイルごとの別の実行環境は、ありません。別のファイルの `globalThis` の値も見えます。
- 誰も受けなかった Promise の拒否が残ると、その呼び出しは失敗になります。
- ハンドラが始めたタイマー（`setTimeout`）は、その呼び出しの中で、すべて終わるまで待ちます。タイマーの中で `Editor` を呼んでも、その呼び出しの文書に対して動きます。終わらないタイマー（`setInterval`）があると、その呼び出しは時間切れになります。タイマーの中で出た例外は、その呼び出しの失敗です。
- 呼び出しの間、Node.js のイベントループは進みません。`Editor` などの呼び出しは、マクロと同じく、答えが返るまで待ちます。

### `.cjs`（CommonJS）

呼び出しのたびに、同意したときに読んだ文面を、新しい CommonJS のモジュールとして、最初から動かします。隣のファイルは `require('./名前.cjs')` で読めます（拡張子は省けます）。

### `.mjs`（ES モジュール）と `.ts`（TypeScript）

関数を `default` で export します。引数は、`Plugin.args` と同じものです。`async` にしても構いません。

```js
// command.mjs
import { greeting } from './lib/greeting.mjs';

export default function (args) {
  Editor.InsText(greeting(Plugin.jack));
}
```

- モジュールは、プロセスの中で、最初の呼び出しのときに 1 回だけ読みます。モジュールの一番上に書いたものは、そのとき 1 回だけ動き、変数に持った値は、次の呼び出しにも残ります。呼び出しのたびに動くのは、export した関数だけです。呼び出しのたびに読み直さないので、呼び出しを重ねても、メモリは増えません。
- 同じプロセスで動くハンドラは、モジュールを共有します。コマンドとイベントが、同じ `.mjs` を import して、値を受け渡せます。
- `.ts` は、ES モジュールです。型は Node.js が取り除きます（`module-typescript`）。`import type` や、型の注釈が書けます。`enum` など、型の取り除きだけでは動かない書き方は使えません。`.cts` は CommonJS の TypeScript です。
- `import` には、拡張子まで書きます（`./lib/util.ts`）。JSON は `import data from './x.json' with { type: 'json' }` で読めます。
- `.mjs` と `.ts` と `.cjs` は、別の言語のハンドラです。`.ts` は、`.mjs` や `.cjs` とは別のプロセスで動きます。

### 同意した文面だけを動かす

プラグインのフォルダーのファイルは、同意したときの SHA-256 と照らして、確かめたものだけが Node.js に渡ります。

- プロセスの最初の呼び出しのとき、フォルダーの中の読み込めるファイル（`.mjs` `.cjs` `.js` `.ts` `.mts` `.cts` `.json`）を、エディターが読み、SHA-256 を同意したときのものと照らし、その文面を Node.js に渡します。Node.js の `import` と `require` は、その文面から読みます。ディスクからは読み直しません。
- 同意したあとで書き換わったファイル、同意のあとで増えたファイルがあるときは、何も動かさずに「`<ファイル>` が、同意したときのファイルと違います」と出ます。
- プロセスが起きたあとでディスクが書き換えられても、動くのは、最初に渡した文面です。書き換えたものは、もう一度「有効にする」を押して同意し直すと、新しいプロセスで動きます。
- `import` と `require` が読めるのは、フォルダーの中のファイルと、Node.js の標準のモジュール（`node:fs` など）だけです。フォルダーの外のファイル、`file:` 以外の場所、パッケージ（`import 'left-pad'` など）は、読めません。ただし、これは `import` と `require` の話です。`node:fs` などで別のファイルを読んだり、`child_process` で別のプログラムを動かしたりすることは、止めません。同意は、フォルダーのコードを動かしてよいという意味で、サンドボックスではありません。「`<名前>` は、同意したプラグインのフォルダーにありません」と出ます。読めるのは、フォルダーの中のファイルと、Node.js の標準のモジュール（`node:fs` など）だけです。
- フォルダーの `.mjs` `.cjs` `.js` `.ts` `.json` の合計が 16 MiB を超えるプラグインは、渡せません。

モジュールに必要な Node.js の機能は、`module.registerHooks`（22.15 以降、23.5 以降）です。それより古い Node.js で `.mjs` と `.ts` を呼ぶと、必要な版を伝えて失敗します。そのような古い Node.js で `.cjs` を動かすと、ハンドラ自身の文面は同意したものですが、`require` で読む隣のファイルは、これまでどおりディスクから読みます（文面を渡す仕組みがないため）。`.ts` の型の取り除きは、`load` の返す形を `module-typescript` にして、Node.js 自身に任せています。`module.stripTypeScriptTypes` は使っていません。

## 根拠

- 起動の引数と名前: `crates/matsu-plugin/src/session.rs` の `Language::default_interpreter`、`start`（`--require`）、`node_strips_types`、`Language::of_file`
- 型の取り除きと版の確認: `crates/matsu-plugin/src/boot/boot.cjs`。`docs/adr/0025-script-languages.md`
- 同期の呼び出し、出力: `boot.cjs`（冒頭の説明）
- 試した版: `docs/poc/json-rpc.md`（「確かめたこと」）。`.github/workflows/ci.yml`（Node.js 22.23）
- プラグインの待ち受け: `boot.cjs` の `servePlugin`（フックは `installVerifiedFiles`）。文面を確かめて渡す所: `crates/matsu-plugin/src/plugin_host.rs` の `PluginHost::begin`、同意した SHA-256 を渡す所: `crates/matsu-ui/src/macros/plugin_call.rs` の `prepare`。Node.js のフック: <https://nodejs.org/api/module.html#moduleregisterhooksoptions>。試験は `crates/matsu-plugin/src/external_plugin_tests.rs`、サンプルは `crates/matsu-plugin/samples/plugins/HelloNode`（`.cjs`）と `HelloModule`（`.mjs` と `.ts`）
- 試験: `crates/matsu-plugin/src/tests.rs`、`crates/matsu-script/tests/quickjs.rs` の `node_extensions_stay_on_node`
