# AppleScript

拡張子は `.applescript` です。macOS だけで動きます。OS が持つ `osascript` を使うので、利用者が入れるものは、ありません。

ほかの OS では、動かしません。「AppleScript のマクロは macOS でだけ動かせます (osascript が要ります)」と出ます。マクロを選ぶダイアログのファイルの種類にも、並びません。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `osascript` |
| 版 | macOS が持つもの |
| プラグイン | 動かせます。下の「プラグインとして動かすとき」 |

## 動き方

- MATSU Editor は、起動用の部分、利用者のスクリプト、`on run` の部分をつなげて、1 つのスクリプトを作り、一時のフォルダーに置いて、`osascript` で動かします。利用者のスクリプトは、`script UserMacro ... end script` の中に入ります。
- AppleScript は、標準入出力を、自分では読み書きできません。そのため、起動用の部分が、AppleScriptObjC（Foundation の `NSFileHandle` と `NSJSONSerialization`）で、MATSU Editor とやり取りします。
- スクリプトは、ふつうの文として書きます。自分の `on run` を持ってもよく、それも動きます。
- 古い Mac のファイルの改行（CR）は、LF にそろえて扱います。行番号は、そろえたあとの行です。
- 呼び方は、`editorCall("InsText", {"文字"})` です。名前は、サクラエディタのマクロの関数の名前です。
- MATSU Editor が渡した値は、`editorInfo("editor.name")`、`editorInfo("document.path")`、`editorInfo("processId")` で読みます。ないときは `missing value` です。
- 出力パネルに出すには、`editorPrint("文字")` を使います。`log` は、標準エラーに行きます。スクリプトの最後の式の値は、出力されません。

## 標準ライブラリ

AppleScript の標準の機能と、AppleScriptObjC で使える macOS の部品が、使えます。

## 文字コード

- スクリプトは UTF-8 で保存します（BOM は取り除きます）。
- 出力は UTF-8 です。ほかの言語のように ASCII に直さず、そのまま書きます（macOS の標準出力は UTF-8 のため）。

## エラー

`Editor` の関数が失敗すると、AppleScript のエラーになります。エラーの番号は、エディタの番号、文は「EditorError 番号: 理由」です。`try ... on error` で受けて、番号と文を読めます。

`osascript` は、エラーの位置を、文字の範囲で報告します。MATSU Editor は、それを、利用者のスクリプトの `名前:行` に直して、出力パネルに出します。利用者のスクリプトの外（起動用の部分）の範囲は、名前だけになります。

## プラグインとして動かすとき

ハンドラの拡張子が `.applescript` のとき、macOS の `osascript` が動かします。macOS だけです。

- Windows と Linux では、動かせません。プラグインの追加と有効にすることは、できます。ただし、そのハンドラを動かす場面が来ると、コマンドを選んだときは、状態バーに「AppleScript のマクロは macOS でだけ動かせます (osascript が要ります)」と出ます。イベントなど、自動で呼ぶ場面では、何も言わずに飛ばします。
- ハンドラの文面は、MATSU Editor が呼び出しのたびに渡します（同意のときに確かめた文面です）。ハンドラは、待ち受けのプログラムの一部としては、コンパイルできません。そのため、待ち受けのプロセスが、起動用の部分、ハンドラ（`script UserMacro ... end script` の中）、`on run` をつなげて 1 つのスクリプトを作り、`NSAppleScript` でコンパイルして動かします。ハンドラの行番号は、ハンドラの文面の行です。
- ハンドラは、待ち受けとは別のスクリプトです。待ち受けの変数は見えません。コンパイルしたスクリプトは、ハンドラの名前で覚えていて、文面が同じ間は、次の呼び出しでも同じものを動かします。そのため、`property` に入れた値は、プロセスが残っている（`UseCache=1` か `Resident=1`）間、残ります（`property n : 0` と `set n to n + 1`）。文面が変わったとき、失敗したときは、コンパイルし直すので、値は初めからです。プロセスが終わっても残したい値は、プラグインのオプション（`editorCall("Plugin.SetOption", {"節", "キー", "値"})`）に入れます。
- `Plugin` などのオブジェクトは、`editorCall` で、名前に `Plugin.`、`Outline.`、`Complement.` を付けて呼びます（`editorCall("Plugin.GetOption", {"節", "キー"})`、`editorCall("Outline.SetTitle", {"見出し"})`、`editorCall("Complement.AddList", {"語"})`）。`Editor` の関数は、マクロと同じです（`editorCall("InsText", {"文字"})`）。

| 読むもの | AppleScript |
|---|---|
| ジャックの名前 | `pluginJack()` |
| プラグインの ID、フォルダー | `pluginId()`、`pluginDir()` |
| 引数の 1 つ（文字か数。ないときは `missing value`） | `pluginArg("n")` |
| `Complement.GetCurrentWord`、`GetOption` | `complementGetCurrentWord()`、`complementGetOption()` |
| `Indent.GetChar` | `indentGetChar()`（`SmartIndent` のときだけ、打った文字を返します。ほかの場面では `""` です） |

- `return` は、そのハンドラを終えるだけです。`error number -128`（スクリプトが「静かにやめる」と言うときの番号）も、そのハンドラを終えるだけで、失敗ではありません。ほかの `error` は、その呼び出しを失敗にします。
- プラグインのフォルダーの中のファイルは、`run script (POSIX file (pluginDir() & "/lib/word.applescript"))` のように読めます。読むファイルは、エディターがフォルダーを確かめた直後に、ディスクから読みます（ハンドラの文面と違い、確かめた文面ではありません）。
- 失敗の文は、エラーの種類と理由と番号と場所です。`execution error: 理由 (番号)`（コンパイルの誤りは `script error: ...`）の次の行に、`  at ハンドラ名:行` が付きます。`Editor` の失敗は、理由が `EditorError 番号: 理由` です。ハンドラの文面は、失敗の文に入りません。失敗した呼び出しの前に書いた出力（`editorPrint`）は、消えません。失敗すると、そのプロセスは終わり、次の呼び出しで、新しく起こします。
- 出力に、改行のない文字を書いて（`editorWrite("文字")`）、そのまま `editorCall` を呼んでも、その文字は 1 行のままです。
- ハンドラが、時間の制限（[時間の制限と失敗](plugin-limits.md)）を超えると、プロセスを終わらせます。

## 根拠

- macOS だけで動くこと、起動: `crates/matsu-plugin/src/session.rs`（`Language::AppleScript`、`NotSupportedHere`、`Language::runs_here`、`start`）
- スクリプトの組み立て、行番号の直し、CR の扱い: `crates/matsu-plugin/src/applescript.rs`
- `editorCall`、`editorInfo`、`editorPrint`、エラーの形、出力: `crates/matsu-plugin/src/boot/boot.applescript`
- サンプル: `crates/matsu-plugin/samples/sample.applescript`
- プラグインの待ち受け、ハンドラごとのスクリプト、失敗の文: `crates/matsu-plugin/src/boot/boot.applescript` の `pluginServe`、`pluginRun`、`pluginFailure`。待ち受けの側のプログラム: `crates/matsu-plugin/src/applescript.rs` の `Program::plugin`。サンプルは `crates/matsu-plugin/samples/plugins/HelloAppleScript`
- 試験: `crates/matsu-plugin/src/tests.rs`、`crates/matsu-plugin/src/applescript.rs` の試験、プラグインは `crates/matsu-plugin/src/external_plugin_tests/emacs_applescript.rs`
