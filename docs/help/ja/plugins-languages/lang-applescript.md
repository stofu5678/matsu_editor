# AppleScript

拡張子は `.applescript` です。macOS だけで動きます。OS が持つ `osascript` を使うので、利用者が入れるものは、ありません。

ほかの OS では、動かしません。「AppleScript のマクロは macOS でだけ動かせます (osascript が要ります)」と出ます。マクロを選ぶダイアログのファイルの種類にも、並びません。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `osascript` |
| 版 | macOS が持つもの |
| プラグイン | 動かせません（動かせるのは QuickJS、Node.js、Python です） |

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

## 根拠

- macOS だけで動くこと、起動: `crates/matsu-plugin/src/session.rs`（`Language::AppleScript`、`NotSupportedHere`、`Language::runs_here`、`start`）
- スクリプトの組み立て、行番号の直し、CR の扱い: `crates/matsu-plugin/src/applescript.rs`
- `editorCall`、`editorInfo`、`editorPrint`、エラーの形、出力: `crates/matsu-plugin/src/boot/boot.applescript`
- サンプル: `crates/matsu-plugin/samples/sample.applescript`
- 試験: `crates/matsu-plugin/src/tests.rs`、`crates/matsu-plugin/src/applescript.rs` の試験
