# プラグインとは

プラグインは、1 つのフォルダーです。中に、定義ファイル `plugin.def` と、スクリプトのファイル（ハンドラ）が入っています。MATSU Editor は、定義ファイルで宣言された場面が来ると、そのハンドラを動かします。

マクロは、利用者が動かしたときに 1 回走って終わります。プラグインは、利用者が動かさなくても、場面が来るたびに呼ばれます。その場面を「ジャック」と呼びます。

プラグインの機能は、既定で切れています。共通設定の「プラグイン」で入れて、プラグインごとに有効にするまで、何も動きません（[プラグインを入れる](plugin-install.md)）。

## ジャックの一覧

ジャックは 11 個あります。定義ファイルの `[Plug]` に、ジャックの名前とハンドラを書きます。

| ジャック | 呼ばれる場面 | ページ |
|---|---|---|
| `Command` | 利用者が、プラグインのコマンドを選んだとき | [コマンド](plugin-commands.md) |
| `EditorStart` | ウィンドウができたとき | [イベント](plugin-events.md) |
| `EditorEnd` | ウィンドウが閉じるとき | [イベント](plugin-events.md) |
| `DocumentOpen` | ファイルを開いたあと | [イベント](plugin-events.md) |
| `DocumentClose` | ファイルを閉じる前 | [イベント](plugin-events.md) |
| `DocumentBeforeSave` | 保存の前 | [イベント](plugin-events.md) |
| `DocumentAfterSave` | 保存のあと | [イベント](plugin-events.md) |
| `Outline` | アウトライン解析をするとき | [アウトライン](plugin-outline.md) |
| `SmartIndent` | 改行などを入力したあと | [スマートインデント](plugin-smart-indent.md) |
| `Complement` | 入力補完を開くとき（タイプ別設定で選んだ 1 つ） | [入力補完](plugin-complement.md) |
| `ComplementGlobal` | 入力補完を開くとき（すべてのタイプ） | [入力補完](plugin-complement.md) |

キー入力のたびに動くジャックは、ありません。`SmartIndent` は、改行と、定義ファイルで名前を挙げた文字だけで呼ばれます。入力補完は、補完を開いたときの 1 回です。

## 今動かせる言語

ハンドラは、拡張子で言語が決まります。

| 拡張子 | 動かすもの |
|---|---|
| `.js` | 同梱の QuickJS（[JavaScript（QuickJS）](lang-quickjs.md)） |
| `.mjs`、`.cjs`、`.ts` | 利用者が入れた Node.js（[JavaScript（Node.js）と TypeScript](lang-node.md)）。`.ts` は Node.js 22.18 以降 |
| `.py` | 利用者が入れた Python（[Python](lang-python.md)） |

どの言語でも、同意、時間の制限、3 回続けて失敗したら止まること、`UseCache=1` と `Resident=1` でプロセスを残すこと、`Plugin` などのオブジェクトは、同じです。外部の言語（Node.js と Python）は、利用者が処理系を入れていないと動きません。そのときは、「`<言語>` を起動できませんでした」と出ます。

1 つのプラグインに、言語の違うハンドラを混ぜることもできます。言語ごとに、別のプロセスで動きます（`.mjs`、`.cjs`、`.ts` は、同じ Node.js のプロセスです）。このとき、失敗の数も言語ごとに数えます。

Ruby、PHP、PowerShell、Perl、Emacs Lisp、AppleScript、Lua、MicroPython、mruby のハンドラを持つプラグインも、追加して有効にできます。ただし、そのハンドラを動かす場面が来ると、動かせません。コマンドを選んだときは、状態バーに「`<ハンドラ名>` は動かせません。プラグインのハンドラは .js、.mjs、.cjs、.ts、.py のどれかにしてください」と出ます。イベントなど、自動で呼ぶ場面では、何も言わずに飛ばします。

次のプラグインは、一覧に「使えません」と出て、有効にできません。

- 種別（`[Plugin]` の `Type`）が `dll` のもの。「DLL プラグインは使えません」と出ます。
- 種別が `wsh`、`script`、`dll` のどれでもないもの。
- ハンドラに VBScript のファイル（`.vbs`、`.vbe`）があるもの。「VBScript は使えません」と出ます。

## サクラエディタとの違い

- 追加しただけでは動きません。利用者が 1 つずつ有効にして、確認の画面で承知する必要があります。サクラエディタは、追加した時点で動きます。
- 呼び出しに時間の制限があります（[時間の制限と失敗](plugin-limits.md)）。サクラエディタにはありません。
- 失敗が 3 回続くと、そのプラグインを止めます。
- `SmartIndent` と入力補完を呼ぶ場面が少なくなっています。サクラエディタは、文字を打つたびに呼びます。
- 動かせる言語が違います。サクラエディタは Windows のスクリプトエンジン（JScript など）で動かし、MATSU Editor は QuickJS で動かします。`WScript` や `ActiveXObject` はありません。これらを使うハンドラは、エラーで終わります。
- プラグインのコマンドやマクロが、そのウィンドウで動いている間に起きたイベントは、どのプラグインにも配りません。
- 1 つのウィンドウに文書が 1 つのサクラエディタと違い、MATSU Editor はタブを持ちます。プラグインは、呼び出しを起こした文書に対して動きます（[ハンドラの中で使えるもの](plugin-api.md)）。

## 根拠

- 設計: `docs/adr/0026-resident-plugins.md` の「決めたこと」と「追記（2026-11-03）」
- ジャックの一覧: `crates/matsu-config/src/plugins/def.rs` の `Jack`
- 使えないプラグイン: 同ファイルの `PluginDef::unsupported`、`is_vbscript`。一覧の言葉は `crates/matsu-ui/src/common_settings/pages/plugin/rows.rs` の `unsupported_text`
- 動かせる言語: `crates/matsu-plugin/src/session.rs` の `Language::serves_plugins`、`start`（`PluginNotSupported`）。`crates/matsu-ui/src/macros/plugin_call.rs` の `prepare`（言語ごとのホスト）
- Node.js と Python の待ち受け: `crates/matsu-plugin/src/boot/boot.cjs`、`boot.py`（`servePlugin`、`_serve_plugin`）
- 自動で呼ぶ場面が何も言わず飛ばすこと: `prepare_event`（`.ok()` で失敗を捨てる）
- コマンドやマクロの間はイベントを配らないこと: `crates/matsu-ui/src/plugin_events.rs` の冒頭の説明、`crates/matsu-ui/src/macros/plugin_call.rs` の `Turn::Taken`
- サクラのソース: `sakura_core/plugin/CJackManager.cpp`（ジャックの名前）、`CWSHPlugin.cpp`（`wsh` の動かし方）
- `docs/sakura-gaps.md` の「プラグイン」の表
