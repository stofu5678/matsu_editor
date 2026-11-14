# ハンドラの中で使えるもの

ハンドラは、JavaScript のファイルです。今は QuickJS で動きます（[JavaScript（QuickJS）](lang-quickjs.md)）。ハンドラには、マクロと同じ `Editor` と `EditorError` と `console` が、最初からあります。そのほかに、プラグイン用のオブジェクトがあります。

## 書き方の決まり

- ファイルは UTF-8 で保存します。先頭の BOM は取り除きます。UTF-8 でないと、動かせません。
- ファイルの中身は、関数の本体として実行します。1 行目はそのままなので、エラーの行番号はファイルのとおりです。
- `let`、`const`、`class`、`var`、関数は、その呼び出しの中だけのものです。同じファイルを何度呼んでも、宣言が重なってエラーになることはありません。
- 次の呼び出しまで残したいものは、`globalThis` に置きます。
- 残したものが見えるのは、同じファイルの次の呼び出しだけです。ハンドラのファイルごとに、別の実行環境があります。別のファイルからは見えません。
- 残せるのは、プロセスが残っているときだけです。`[Wsh]` の `UseCache=1` も `[Matsu]` の `Resident=1` も指定していないプラグインは、呼び出しのたびに新しいプロセスで動くので、何も残りません（[plugin.def の書き方](plugin-def.md)）。
- 既定は、厳密でないモード（sloppy モード）です。サクラエディタ用のプラグインは、宣言していない名前へ代入するものがあるためです。ファイルの先頭に `"use strict"` と書くと、そのファイルは厳密なモードになります。
- `this` は、`globalThis` です。
- `require` と `module` はありません。

## `Editor`

マクロと同じ関数が、同じ名前と引数で使えます（「マクロの関数」の章）。呼び出しを起こした文書に対して動きます。

- 呼び出しの間、対象の文書は決まっています。利用者がほかのタブに切り替えても、変わりません。
- その文書が閉じられたり、ウィンドウが別の文書を読み込んだりすると、`Editor` の呼び出しはエラーを返します。プロセスも止めます。
- プラグインが `Editor` でした操作は、キーマクロに記録しません。
- 呼び出し 1 回で文書にしたことは、元に戻す操作で、まとめて 1 回で戻せます。ただし、入力補完の呼び出しと、利用者が IME で変換中のときの呼び出しは、まとめません。
- `MessageBox`、ファイルを選ぶダイアログ、`Sleep` のように、利用者や時間を待つ関数は、プラグインの中ではエラーになります。

## `Plugin`

| 名前 | 内容 |
|---|---|
| `Plugin.id` | プラグインの `Id` |
| `Plugin.dir` | プラグインのフォルダーのパス |
| `Plugin.jack` | 今呼ばれているジャックの名前 |
| `Plugin.args` | 呼び出しの引数（下の表） |
| `Plugin.GetPluginDir()` | プラグインのフォルダーのパス |
| `Plugin.GetDef(section, key)` | `plugin.def` の値。なければ空の文字列 |
| `Plugin.GetOption(section, key)` | オプションの値。値のファイルになければ、`plugin.def` の既定の値。どちらにもなければ空の文字列。呼ぶたびにファイルを読み直します |
| `Plugin.SetOption(section, key, value)` | オプションの値を、すぐにファイルへ書きます。`value` は省けません。文字列でない値は、その JSON の形で書きます |
| `Plugin.GetString(n)` | `[String]` の `S[n]`。`n` が 0 のときは、言語の名前（`ja_JP`）。1 から 99 以外は空の文字列 |
| `Plugin.GetCommandNo()` | `Command` ジャックで、呼ばれたコマンドの番号。ほかでは 0 |
| `Plugin.AddCommand(handler, label, icon)` | コマンドを 1 つ足します。下を参照 |

`Plugin.args` の中身は、ジャックで決まります。

| ジャック | `Plugin.args` |
|---|---|
| `Command` | `{command: 呼ばれたコマンドの番号}` |
| `SmartIndent` | `{char: 入力した文字}` |
| `Complement`、`ComplementGlobal` | `{word: キャレットの前の単語, option: 補完のオプション}` |
| そのほか | `{}` |

### `Plugin.AddCommand`

実行中のプラグインに、コマンドを足します。`handler` は、プラグインのフォルダーの中のファイルの名前です。フォルダーの外を指すと、エラーです。`label` を省くと、`handler` が名前になります。

- 足したコマンドは、メニューとキー割り当てとツールバーに出ます（[コマンド](plugin-commands.md)）。
- 同じ `handler` と `label` を何度足しても、1 つです。ウィンドウが開くたびに足すプラグインのためです。
- 1 つのプラグインのコマンドは、49 個までです。超えるとエラーです。
- 足したコマンドは、プラグインを無効にするか外すまで残ります。`plugin.def` が同じ番号のコマンドを持つようになったときは、`plugin.def` のものになります。

## `Outline`、`Indent`、`Complement`

| オブジェクト | いつあるか | 内容 |
|---|---|---|
| `Outline` | `Outline` ジャックの呼び出しの中だけ | `AddFuncInfo`、`AddFuncInfo2`、`AddFuncInfo3`、`AddFuncInfo4`、`SetTitle`、`SetListType`、`SetLabel`。[アウトライン](plugin-outline.md)を参照 |
| `Indent` | `SmartIndent` ジャックの呼び出しの中 | `GetChar()`。入力した文字を返します。Enter のときは `"\r"` です。[スマートインデント](plugin-smart-indent.md)を参照 |
| `Complement` | プラグインの中 | `GetCurrentWord()`、`GetOption()`、`AddList(word)`。[入力補完](plugin-complement.md)を参照 |

`Outline` のオブジェクトを `Outline` ジャックの外で使うと、エラーになります。`Complement.AddList` を、補完の呼び出しの外で使ったときも、エラーです。

## 終わり方とエラー

- ハンドラが終わったあと、`async` 関数や `.then` の処理も、終わるまで待ちます。誰も受けなかった拒否（rejection）は、エラーとして扱います。
- ハンドラが例外を投げたり、構文の間違いがあったりすると、行番号つきのエラーを、出力パネルに出します（[時間の制限と失敗](plugin-limits.md)）。
- `console.log` などの出力は、出力パネルに出ます。`console.error` と `console.warn` は、標準エラーの出力として扱います。ただし、イベントや補完などの自動の呼び出しは、うまくいって何も出力しなかったときは、出力パネルを出しません。

## サクラエディタとの違い

- 呼び出しを起こした文書が決まっています。サクラエディタは、ウィンドウに文書が 1 つなので、この問題がありません。
- `Plugin.AddCommand` で足したコマンドは、ウィンドウごとではなく、MATSU Editor 全体で 1 つです。
- ダイアログと `Sleep` が使えません。
- `WScript` と `ActiveXObject` がありません。

## 根拠

- 実行の決まり（関数の本体、`globalThis`、sloppy モード）: `crates/matsu-script/src/quickjs.rs` の `run_handler`、`QuickJs::serve`、`Vm::finish`。試験は `crates/matsu-script/tests/plugin_host.rs`
- `Plugin` `Outline` `Indent` `Complement` のオブジェクト: `crates/matsu-script/src/quickjs.js`（`bindCalls`、`begin`、`Complement`）、`PLUGIN_CALLS`、`OUTLINE_CALLS`、`COMPLEMENT_CALLS`
- `Plugin.*` の答え方: `crates/matsu-ui/src/macros/plugin_call.rs` の `DefApi`、`DefApi::add_command`、`prepare`（引数の形）
- 文書の固定と、元に戻す操作のまとまり: 同ファイルの `run`、`release_document`、`EditorView::answer_plugin_request`。キーマクロに記録しないこと: `answer_plugin_request` の `suspended`
- ダイアログと `Sleep`: `crates/matsu-plugin/src/functions/window.rs`（ウィンドウがないとき `UNAVAILABLE`）、`crates/matsu-plugin/src/bridge.rs`。`answer_plugin_request` は `bridge::call_with` だけを呼びます
- ハンドラのファイルの読み方: `crates/matsu-plugin/src/plugin_host.rs` の `read_handler`
- サクラのソース: `sakura_core/plugin/CPluginIfObj.h`、`COutlineIfObj.h`、`CSmartIndentIfObj.h`、`CComplementIfObj.h`
