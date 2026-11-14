# すべての言語に共通のこと

言語ごとのページの前に、どの言語にも共通する動きを説明します。言語と拡張子の対応は、「マクロの使い方」の章にあります。

## 動かし方

スクリプトは、MATSU Editor とは別のプロセスで動きます。止まっても、無限に繰り返しても、巻き込まれるのはそのプロセスだけです。

- 同梱の言語（Lua、QuickJS、MicroPython、mruby）は、MATSU Editor の実行ファイルの隣にある `matsu-script` が動かします。隣になければ、`PATH` から探します。
- 外部の言語は、利用者が入れた処理系を、`PATH` から探して起動します。

| 言語 | 起動する名前 |
|---|---|
| JavaScript（Node.js）、TypeScript | `node` |
| Python | Windows は `python`、そのほかは `python3` |
| Ruby | `ruby` |
| PHP | `php` |
| PowerShell | `pwsh`。Windows で `pwsh.exe` が `PATH` になければ `powershell`（Windows PowerShell 5.1） |
| Perl | `perl` |
| Emacs Lisp | `emacs` |
| AppleScript | `osascript` |

処理系の場所を変える設定は、いまのところありません。`PATH` で見つかるものが使われます。macOS で Finder や Dock から起動したときは、ログインシェルの `PATH` を使います（起動の直後で、まだ読めていないときは、起動したときの `PATH`）。

Windows では、処理系のコンソールの窓は出ません。

起動できなかったときは、出力パネルと状態バーに「`<言語>` を起動できませんでした」と、理由が出ます。Windows で `python` が Microsoft Store の案内だけのとき（終了コード 9009）は、「Python が見つかりません。インストールを確かめてください」も出ます。

## 文字コード

- スクリプトのファイルは、UTF-8 で保存します。
  - 同梱の言語、PowerShell、Perl、Emacs Lisp、AppleScript は、MATSU Editor がファイルを読んで、中身を処理系に渡します。UTF-8 でないと、「`<ファイル>` を読めませんでした (UTF-8 で保存してください)」と出て、動きません。先頭の BOM は取り除きます。
  - Node.js、Python、Ruby、PHP は、処理系がファイルを読みます。UTF-8 で保存してください。
- スクリプトが出力した文字は、UTF-8 として読みます。UTF-8 でないバイトは、`U+FFFD`（�）になります。処理系の出力を UTF-8 にするための設定は、言語のページにあります。
- 行番号と桁は 1 から数えます。桁は UTF-16 の単位です（「マクロの使い方」の章）。

## 出力とエラー

出力パネルに、次のものが出ます。

- スクリプトの標準出力は、そのままの行として。
- 標準エラーは、エラーとして。
- 開始と終わりの知らせ。終わりは、「正常に終了しました (終了コード 0)」「エラーで終了しました (終了コード N)」「時間切れで中止しました (30 秒)」「中断しました」のどれかです。

改行のない出力は、改行が出るか、スクリプトが終わるまで、出力パネルに出ません。

捕まえなかったエラーは、行番号つきで標準エラーに出て、スクリプトは終了コード 1 などで終わります。行番号は、どの言語でも、スクリプトのファイルの行です。エラーの文の形は、言語ごとに違います。

`Editor` の関数の失敗は、各言語のページにある形の例外になります（`EditorError`）。

## 時間の制限と止め方

- マクロは、30 秒で止まります。時間は固定です。MATSU Editor がプロセスを終わらせます。
- ダイアログ（`MessageBox`、ファイルを選ぶダイアログ）と `Sleep` を待っている間は、時間に数えません。
- 出力パネルの「中止」で、いつでも止められます。プロセスを終わらせるだけなので、ループの最中でも止まります。
- 共通設定の「マクロ」の「キャンセル確認待ち時間」（既定 10 秒）がたつと、止めるかどうかを聞く箱が出ます（「マクロの使い方」の章）。
- キーマクロから呼んだスクリプトが、失敗する、時間切れになる、中止されるのどれかになると、キーマクロの続きも止まります。

プラグインは、この 30 秒の制限を使いません。場面ごとの制限と、失敗の扱いは、[時間の制限と失敗](plugin-limits.md)にあります。プラグインを動かせる言語は、QuickJS（`.js`）、Node.js（`.mjs`、`.cjs`、`.ts`）、Python（`.py`）です。ほかの言語は、まだ動かせません。

## `Editor` の関数

どの言語も、同じ一覧（サクラエディタのマクロの関数と、MATSU Editor 独自のもの）から、`Editor` の関数を作ります。名前は大文字と小文字を区別します。数字で始まる名前（`1PageUp` など）は、メソッドの形では書けない言語があります。その言語では、名前を文字列で渡す呼び出し方を使います。

同梱の言語では、引数の最後の `nil`、`undefined`、`None` は、省略として扱います。

## 根拠

- 起動と処理系の名前: `crates/matsu-plugin/src/session.rs` の `Language::default_interpreter`、`powershell_interpreter`、`start`、`StartError`
- 文字コード: 同ファイルの `read_script`、`read_lines`、`start`（`PYTHONUTF8`）
- 時間と止め方: 同ファイルの `DEFAULT_TIMEOUT`、`Session::hold`、`Session::cancel`。UI の側は `crates/matsu-ui/src/macros/mod.rs`（`start_script`、`ask_to_stop_after_wait`、`on_host_event`）。言葉は `crates/matsu-plugin/src/lib.rs` の `describe`
- macOS の `PATH`: `crates/matsu-platform/src/login_path/mod.rs`
- 同梱の `matsu-script`: `crates/matsu-script/src/main.rs`、`engine.rs`
- 数字で始まる名前: `docs/poc/json-rpc.md`（「基本 API」）
