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
| `.lua` | 同梱の Lua（[Lua](lang-lua.md)） |
| `.upy` | 同梱の MicroPython（[MicroPython](lang-micropython.md)）。`.py` は、利用者が入れた Python です |
| `.mruby` | 同梱の mruby（[mruby](lang-mruby.md)）。`.rb` は、利用者が入れた Ruby です |
| `.cjs` | 利用者が入れた Node.js（[JavaScript（Node.js）と TypeScript](lang-node.md)）。`.mjs`（ES モジュール）と `.ts`（TypeScript）も動かせます |
| `.py` | 利用者が入れた Python（[Python](lang-python.md)） |
| `.rb` | 利用者が入れた Ruby（[Ruby](lang-ruby.md)） |
| `.php` | 利用者が入れた PHP（[PHP](lang-php.md)） |
| `.ps1` | 利用者が入れた PowerShell（[PowerShell](lang-powershell.md)）。`pwsh`（PowerShell 7）を探し、Windows で `pwsh.exe` が `PATH` になければ Windows PowerShell 5.1 を使います |
| `.pl` | 利用者が入れた Perl（[Perl](lang-perl.md)） |

どの言語でも、同意、時間の制限、3 回続けて失敗したら止まること、`UseCache=1` と `Resident=1` でプロセスを残すこと、`Plugin` などのオブジェクトは、同じです。外部の言語（Node.js、Python、Ruby、PHP、PowerShell、Perl、Emacs Lisp）は、利用者が処理系を入れていないと動きません。AppleScript は、macOS が `osascript` を持っているので、macOS では入れるものがありません。Windows と Linux では動かせず、コマンドを選ぶと、状態バーに「AppleScript のマクロは macOS でだけ動かせます (osascript が要ります)」と出ます。イベントなど、自動で呼ぶ場面では、何も言わずに飛ばします。そのときは、「`<言語>` を起動できませんでした」と出ます。

1 つのプラグインに、言語の違うハンドラを混ぜることもできます。言語ごとに、別のプロセスで動きます。失敗の数は、言語に関わらず、プラグインで数えます。3 回続けて失敗すると、プラグインが止まります。

ハンドラの拡張子が、上の表にないとき（`.vbs` を除く）は、コマンドを選んだときに、状態バーに「`<ハンドラ名>` は動かせません。プラグインのハンドラは .js、.cjs、.mjs、.ts、.py、.rb、.php、.ps1、.pl、.el、.applescript (macOS だけ)、.lua、.mruby、.upy のどれかにしてください」と出ます。イベントなど、自動で呼ぶ場面では、何も言わずに飛ばします。

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
- プラグインのコマンドが動いている間や、マクロが 1 つの呼び出しを実行している間に起きたイベントは、どのプラグインにも配りません。マクロが保存や読み込みの終わりを待っている間は、その保存や読み込みのイベントを配ります。
- 1 つのウィンドウに文書が 1 つのサクラエディタと違い、MATSU Editor はタブを持ちます。プラグインは、呼び出しを起こした文書に対して動きます（[ハンドラの中で使えるもの](plugin-api.md)）。

## 根拠

- 設計: `docs/adr/0026-resident-plugins.md` の「決めたこと」と「追記（2026-11-03）」
- ジャックの一覧: `crates/matsu-config/src/plugins/def.rs` の `Jack`
- 使えないプラグイン: 同ファイルの `PluginDef::unsupported`、`is_vbscript`。一覧の言葉は `crates/matsu-ui/src/common_settings/pages/plugin/rows.rs` の `unsupported_text`
- 動かせる言語: `crates/matsu-plugin/src/session.rs` の `Language::FILE_EXTENSIONS`、`Language::of_file`。`crates/matsu-ui/src/macros/plugin_call.rs` の `prepare`（言語ごとのホスト）。同梱の言語の待ち受け: `crates/matsu-script/src/plugin.rs`
- Node.js、Python、Ruby、PHP、PowerShell、Perl、Emacs Lisp、AppleScript の待ち受け: `crates/matsu-plugin/src/boot/boot.cjs`、`boot.py`、`boot.rb`、`boot.php`、`boot.ps1`、`boot.pl`、`boot.el`、`boot.applescript`（`servePlugin`、`_serve_plugin`、`MatsuPlugin.serve`、`nextInvoke`、`Start-MatsuPlugin`、`serve`、`plugin--serve`、`pluginServe`）
- 自動で呼ぶ場面が何も言わず飛ばすこと: `prepare_event`（`.ok()` で失敗を捨てる）
- コマンドや、マクロの呼び出しの間はイベントを配らないこと（マクロが待っている間は配る）: `crates/matsu-ui/src/plugin_events.rs` の冒頭の説明、`crates/matsu-ui/src/macros/plugin_call.rs` の `Turn::Taken`
- サクラのソース: `sakura_core/plugin/CJackManager.cpp`（ジャックの名前）、`CWSHPlugin.cpp`（`wsh` の動かし方）
- `docs/sakura-gaps.md` の「プラグイン」の表
