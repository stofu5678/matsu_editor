# マクロの使い方

マクロは、エディタの操作を文書に対して順に行うファイルです。繰り返しの作業を 1 回の実行にまとめられます。MATSU Editor のマクロには、2 つの形があります。

- **キーマクロ**（`.mac`）。操作を記録して、あとで再生します。プログラムを書く必要はありません。[キーマクロ](key-macro.md)に書きました。
- **スクリプト**。Lua や JavaScript、Python などの言語で、操作を書きます。条件分けや繰り返しが使えます。

どちらも、サクラエディタのマクロの関数（`InsText` や `GoFileTop` など）を同じ名前で呼びます。関数の一覧は「マクロの関数」の章にあります。

## 使える言語

拡張子で、言語と動かし方が決まります。

| 拡張子 | 言語 | 動かし方 | 利用者が入れるもの |
|---|---|---|---|
| `.mac` | キーマクロ | 本体 | なし |
| `.lua` | Lua | 同梱の `matsu-script` | なし |
| `.js` | JavaScript（QuickJS） | 同梱の `matsu-script` | なし |
| `.upy` | MicroPython | 同梱の `matsu-script` | なし |
| `.mruby` | mruby | 同梱の `matsu-script` | なし |
| `.mjs` `.cjs` | JavaScript（Node.js） | 外部 | Node.js |
| `.ts` | TypeScript | 外部（Node.js が型を取り除く） | Node.js。22 系は 22.18 以降、23 系は 23.6 以降、24 以降はすべて（23.0〜23.5 と 22.18 より前は、エラーになります） |
| `.py` | Python | 外部 | Python |
| `.rb` | Ruby | 外部 | Ruby |
| `.php` | PHP | 外部 | PHP |
| `.ps1` | PowerShell | 外部 | PowerShell |
| `.pl` | Perl | 外部 | Perl |
| `.el` | Emacs Lisp | 外部 | Emacs 30 以降 |
| `.applescript` | AppleScript | 外部（macOS だけ） | なし（OS が持つ `osascript`） |

- 「同梱」の言語は、何も入れなくても動きます。MATSU Editor の隣にある `matsu-script` という別のプログラムが動かします。
- 「外部」の言語は、利用者が入れた処理系を使います。処理系は `PATH` から探します（Windows の Python は `python`、そのほかの OS の Python は `python3`、Node.js は `node`）。
- `.js` は同梱の QuickJS で動きます。Node.js で動かしたいときは、拡張子を `.mjs` にします。
- `.ts` の拡張子は、小文字だけが TypeScript です。`.TS` は TypeScript のマクロとして扱いません。
- VBScript（`.vbs`）は動きません。書き換え方は[VBScript から書き換える手引き](from-vbscript.md)にあります。
- 外の処理系は、マクロを動かすたびに、別のプロセスとして起動します。マクロが止まらなくなったときも、そのプロセスを終わらせれば済みます。

## 動かし方

次のどれかで動かします。

- メニューの「ツール」の「名前を指定してマクロ実行...」。ファイルを選ぶと動きます。ファイルの種類の欄には、この OS で動く言語だけが並びます。
- 共通設定の「マクロ」のページに登録して、キー・メニュー・ツールバーから呼ぶ。登録は 50 個までです。ここでは、ファイルを開いたあと、タイプを変えたあと、保存の前に、自動で動かすマクロも選べます。
- コマンドラインの `-M=` で、起動したあとに動かす。`-MTYPE=` で、`-M=` に渡したものがマクロのコードそのものか、ファイルかを指定できます。

スクリプトが動いている間は、画面の下の出力パネルに、スクリプトの出力と、エディタからの知らせ（開始、終わり、エラー）が出ます。パネルの「中止」で止められます。共通設定の「マクロ」にある「キャンセル確認待ち時間」（既定は 10 秒）がたつと、止めるかどうかを聞く箱も出ます。

エラーには行番号が付きます。

## スクリプトの書き方

スクリプトには、`Editor` という名前が最初からあります。`require` や `import` は書きません。`Editor` の関数は、サクラのマクロの関数と同じ名前と引数です。

| 言語 | 呼び方の例 |
|---|---|
| Lua、JavaScript、TypeScript、Python、MicroPython | `Editor.InsText("文字")` |
| Ruby、mruby | `Editor.InsText("文字")`（括弧は省けます） |
| PHP | `Editor::InsText("文字");` |
| Perl | `Editor->InsText("文字");` |
| PowerShell | `$Editor.InsText("文字")` |
| Emacs Lisp | `(editor-ins-text "文字")`。名前はケバブケースで、前に `editor-` が付きます。名前が決まっていない呼び出しは `(editor-call "名前" ...)` |
| AppleScript | `editorCall("InsText", {"文字"})` |

- 行番号と桁は 1 から数えます。桁は UTF-16 の単位で数えるので、絵文字などは 2 と数えます。
- 値を返す関数は、返した値をそのまま受け取れます。返さない関数の結果は、空の値です。
- 引数が間違っているなど、エディタが拒否したときは、言語ごとの形でエラーになります。
  - Lua、JavaScript、TypeScript、Python、MicroPython、Ruby、mruby、PHP、Perl: 例外 `EditorError` が起きます。`code`（番号）と `message`（理由）を持ちます。Lua では `pcall` で受けて、`code` と `message` を読みます
  - PowerShell: `Get-EditorErrorCode $_` で番号を読みます
  - Emacs Lisp: シグナル `editor-error` が起きます。データは、番号と理由の 2 つの並びです（`condition-case` で受けて、データを読みます）
  - AppleScript: AppleScript のエラーが起きます。エラー番号がエディタの番号で、文面は `EditorError 番号: 理由` の形です
- `print` や `console.log` など、その言語の標準の出力は、出力パネルに出ます。

Lua の短い例です。1 行目に題を足し、全体の行数を出力します。

```lua
Editor.GoFileTop()
Editor.InsText("# memo\n")
print("lines:", Editor.GetLineCount(0))
```

言語ごとの、もう少し長い例は[マクロのサンプル](samples.md)にあります。

## サクラエディタとの違い

- スクリプトを動かす言語が違います。サクラは Windows のスクリプトエンジン（JScript と VBScript など）に任せます。MATSU Editor は、上の表の言語を、別のプロセスで動かします。
- PPA（`.ppa`）のマクロは動かしません。
- マクロが動いている間も、文書を打てます。サクラは、マクロの間、ウィンドウを止めます。
- マクロを止める手段があります（パネルの「中止」）。サクラのマクロには、その手段がありません。
- ダイアログやパネルを開く関数は、閉じるまで待ちます。待っている間は、時間の制限に数えません。
- サクラのマクロのうち、`WScript.*` や `ActiveXObject` のような、Windows のスクリプトの仕組みに頼る部分は動きません。

## 根拠

- 言語と拡張子: `crates/matsu-plugin/src/session.rs` の `Language::FILE_EXTENSIONS`、`Language::default_interpreter`、`Language::of_file`。ADR-0007（`docs/adr/0007-macros-and-plugins.md`）、ADR-0025（`docs/adr/0025-script-languages.md`）
- 同梱の言語を `matsu-script` が動かすこと: `crates/matsu-script/src/lua.rs`、`crates/matsu-script/src/quickjs.rs`
- 動かし方のメニューと登録: `crates/matsu-commands/src/sakura/main_menu.rs`（「ツール」の項目）、`crates/matsu-ui/src/common_settings/pages/macros.rs`、`crates/matsu-ui/src/macros/registered.rs`
- `-M=` と `-MTYPE=`: `crates/matsu-ui/src/command_line.rs`、`crates/matsu-ui/src/ui_commands/macros/exec_ext_macro.rs`
- 拡張子を受け付けない扱い: `crates/matsu-ui/src/macros/mod.rs` と `crates/matsu-ui/src/macros/tests.rs` の `an_unknown_extension_is_refused`
- 出力パネル、中止、キャンセル確認待ち時間: `crates/matsu-ui/src/macros/output.rs`、`crates/matsu-ui/src/macros/mod.rs`（`CancelWait`）
- 言語ごとの呼び方: `crates/matsu-plugin/samples/` の `sample.*`、`crates/matsu-script/src/lua.rs`（`EditorError` の作り）、`crates/matsu-plugin/src/boot/boot.el`（`editor-call`）、`crates/matsu-plugin/src/boot/boot.applescript`（`editorCall`）。Node.js の版: `crates/matsu-plugin/src/session.rs` の `node_strips_types`、`crates/matsu-plugin/src/boot/boot.cjs`
- 桁の数え方: `crates/matsu-plugin/src/api.rs` の `MoveCursor`
- サクラのソース: `sakura_core/macro/CWSHManager.cpp`（`Editor` オブジェクトを渡す）、`sakura_core/macro/CSMacroMgr.cpp`
