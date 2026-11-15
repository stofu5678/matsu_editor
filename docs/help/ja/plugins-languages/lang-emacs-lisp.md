# Emacs Lisp

拡張子は `.el` です。利用者が入れた Emacs で動かします。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `emacs`（`PATH` から探します） |
| 版 | Emacs 30 以降。JSON の関数が、30 から本体に入ったためです。29 でも、JSON に対応してビルドしたものなら動きます。足りないときは、「Emacs Lisp macros need Emacs 30 or later」と出て、終わります |
| プラグイン | 動かせます。下の「プラグインとして動かすとき」 |

## 動き方

- `emacs -Q --batch --load boot.el` で起動します。`-Q` は、利用者の設定ファイル（init ファイル）とサイトの設定を読まない指定です。`--batch` は、画面を出さない指定です。
- スクリプトは、MATSU Editor が読んで、中身を渡します。1 つ 1 つのトップレベルの式（form）を順に読んで、評価します。エラーは、その式が始まった行を言います。
- 評価は、レキシカルバインディングで行います。
- 式を読むバッファーと、評価するバッファーは、別です。スクリプトが `goto-char` や `set-buffer` や `erase-buffer` を使っても、残りの式を読む処理は乱れません。
- `Editor` の関数は、ケバブケースで、前に `editor-` が付きます（`(editor-ins-text "文字")`）。名前が決まっていない呼び出しは `(editor-call "名前" ...)` です。`require` は要りません。

## 標準ライブラリ

利用者の Emacs の Lisp の関数と、Emacs 本体に入っているライブラリが、そのまま使えます。`-Q` なので、利用者の設定ファイルは読み込まれず、そこで入れたものは使えません。

## 文字コード

- スクリプトは UTF-8 で保存します（BOM は取り除きます）。
- 標準出力と標準エラーは、UTF-8 に設定します。OS のコードページにかかわらず、そうなります。

## エラー

`Editor` の関数が失敗すると、`editor-error` のシグナルが上がります。データは、番号と理由です。

受けなかったエラーは、`スクリプト名:行: メッセージ` の形で、標準エラーに出ます。`Editor` のエラーは、`EditorError 番号: 理由` の形です。スクリプトは、終了コード 1 で終わります。

## プラグインとして動かすとき

ハンドラの拡張子が `.el` のとき、利用者の Emacs が動かします。起動のしかたは、マクロと同じです（`emacs -Q --batch --load boot.el`）。

- ハンドラの文面は、MATSU Editor が呼び出しのたびに渡します（同意のときに確かめた文面です）。マクロと同じに、式を 1 つずつ読んで、評価します。式を読むバッファーと、評価するバッファーは別で、評価するバッファーは、呼び出しごとに作り直します。エラーは、その式が始まった行を言います。
- `Plugin` などのオブジェクトは、変数と関数です。ジャックの名前は `plugin-jack`、引数は `plugin-args`（JSON のオブジェクトは alist です。`(alist-get 'n plugin-args)`）、プラグインの ID は `plugin-id`、フォルダーは `plugin-dir` です。`Editor` の関数は、マクロと同じです（`(editor-ins-text "文字")`）。

| 呼び出し | Emacs Lisp |
|---|---|
| `Plugin.GetPluginDir`、`GetDef`、`GetOption`、`SetOption`、`GetString`、`GetCommandNo`、`AddCommand` | `(plugin-get-plugin-dir)`、`(plugin-get-def ...)`、`(plugin-get-option "節" "キー")`、`(plugin-set-option ...)`、`(plugin-get-string ...)`、`(plugin-get-command-no ...)`、`(plugin-add-command ...)` |
| `Outline.AddFuncInfo`、`AddFuncInfo2`、`AddFuncInfo3`、`AddFuncInfo4`、`SetTitle`、`SetListType`、`SetLabel` | `(plugin-outline-add-func-info ...)`、`(plugin-outline-add-func-info2 ...)`、`(plugin-outline-add-func-info3 ...)`、`(plugin-outline-add-func-info4 ...)`、`(plugin-outline-set-title ...)`、`(plugin-outline-set-list-type ...)`、`(plugin-outline-set-label ...)` |
| `Complement.GetCurrentWord`、`GetOption`、`AddList` | `(plugin-complement-get-current-word)`、`(plugin-complement-get-option)`、`(plugin-complement-add-list "語")` |
| `Indent.GetChar` | `(plugin-indent-get-char)`（`SmartIndent` のときだけ、打った文字を返します。ほかの場面では `""` です） |

- 引数の最後の `nil` は、省略として扱います。
- ハンドラは、MATSU Editor の側の変数を見ません。式は、空のレキシカル環境で評価するので、ハンドラの `let` の変数は、その式の中だけのものです。宣言のない変数に `setq` で値を入れると、グローバル変数になり、プロセスが残っている（`UseCache=1` か `Resident=1`）間、次の呼び出しに残ります。残したいものは、`(defvar 名前 初期値)` と `setq` で持たせます（`defvar` は、値のある変数をもとに戻しません）。`defun` の関数も残ります。
- ハンドラ自身の終わり方は、そのハンドラだけを終えます。`(kill-emacs)` は、その呼び出しを終えるだけで、失敗ではありません（`unwind-protect` の後始末は動きます）。`(throw 'plugin-exit nil)` も同じです。ハンドラ自身の `condition-case` は、`kill-emacs` を捕まえません。
- `(require 'foo)` と `(load ...)` で、プラグインのフォルダーの中のファイルを読めます（`load-path` の先頭が、プラグインのフォルダーです）。読むファイルは、エディターがフォルダーを確かめた直後に、ディスクから読みます（ハンドラの文面と違い、確かめた文面ではありません。Python、Ruby、PHP が import するファイルを読むのと同じです）。`require` したファイルは、一度読むと、読み直しません。
- 失敗の文は、エラーの種類と理由と場所です。`種類: 理由` の次の行に、`  at ハンドラ名:行` が付きます（`void-variable: ...`、`end-of-file: End of file during parsing`、`quit: Quit` など）。`Editor` の失敗は、`EditorError 番号: 理由` です。ハンドラの文面は、失敗の文に入りません。失敗した呼び出しの前に書いた出力は、消えません。失敗すると、そのプロセスは終わり、次の呼び出しで、新しく起こします。
- 出力に、改行のない文字を書いて、そのまま `Editor` の関数を呼んでも、その文字は 1 行のままです。
- ハンドラが、時間の制限（[時間の制限と失敗](plugin-limits.md)）を超えると、プロセスを終わらせます。

## 根拠

- 起動の引数: `crates/matsu-plugin/src/session.rs` の `Language::EmacsLisp`、`start`
- 式ごとの評価、別のバッファー、文字コード、エラーの形、版の確認: `crates/matsu-plugin/src/boot/boot.el`
- Emacs 30 以降を対象にした決定: `docs/adr/0007-macros-and-plugins.md`
- プラグインの待ち受け: `crates/matsu-plugin/src/boot/boot.el` の `plugin--serve`、`plugin--invoke`、`plugin--run-handler`、`plugin--describe`、`editor--eval-forms`。サンプルは `crates/matsu-plugin/samples/plugins/HelloEmacsLisp`
- 試験: `crates/matsu-plugin/src/tests.rs`、プラグインは `crates/matsu-plugin/src/external_plugin_tests/emacs_applescript.rs`
