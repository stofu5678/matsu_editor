# Emacs Lisp

拡張子は `.el` です。利用者が入れた Emacs で動かします。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `emacs`（`PATH` から探します） |
| 版 | Emacs 30 以降。JSON の関数が、30 から本体に入ったためです。29 でも、JSON に対応してビルドしたものなら動きます。足りないときは、「Emacs Lisp macros need Emacs 30 or later」と出て、終わります |
| プラグイン | 動かせません（動かせるのは QuickJS、Node.js、Python です） |

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

## 根拠

- 起動の引数: `crates/matsu-plugin/src/session.rs` の `Language::EmacsLisp`、`start`
- 式ごとの評価、別のバッファー、文字コード、エラーの形、版の確認: `crates/matsu-plugin/src/boot/boot.el`
- Emacs 30 以降を対象にした決定: `docs/adr/0007-macros-and-plugins.md`
- 試験: `crates/matsu-plugin/src/tests.rs`
