# 外部HTMLヘルプ

利用者が設定した HTML ヘルプのファイルを、言葉を引いて開きます。

- 機能名: `F_EXTHTMLHELP`
- メニュー: 「ヘルプ」→「外部HTMLヘルプ」
- 既定のキー: `F1`
- マクロ: `ExtHtmlHelp(ファイル, 言葉)`（キーマクロには記録されません）

## 引数（マクロ）

| 引数 | 意味 |
|---|---|
| ファイル | 開く HTML ヘルプのファイル。省略するか空にすると、設定のファイルを使う |
| 言葉 | 引く言葉。省略するか空にすると、選択範囲の最初の行、なければカーソル位置の単語を使う |

メニューやキーから動かしたときは、引数はありません。ファイルは設定のもの、言葉は文書から決まります。

## 使うファイル

引数のファイルがないときは、次の順に、最初に空でないものを使います。

1. 文書のタイプの「タイプ別設定」→「支援」の「外部HTMLヘルプの設定」
2. 「共通設定」→「支援」の「外部HTMLヘルプの設定」（`matsu.ini` の `szExtHtmlHelp`）

どちらも空のときは、「外部HTMLヘルプが設定されていません。今すぐ設定しますか?」と聞きます。「はい(Y)」を選ぶと、「共通設定」を「支援」のページで開き、この機能は終わります。設定したあとで、もう一度動かしてください。「いいえ(N)」と「キャンセル」は、何もしません。

相対パスのファイルは、`matsu.ini` のあるフォルダ、実行ファイルのあるフォルダの順に探します。

## 開き方

- ファイルの拡張子は見ません。どんなファイルでも、HTML ヘルプとして言葉を引きます。
- 言葉を文書から取るときは、259 文字までです。
- 言葉引きができるのは Windows だけです。Windows 以外と、言葉引きに失敗したときは、ファイルを OS のアプリで開き、ステータスバーに「この環境ではヘルプをキーワードで検索できないため、ファイルを開きました」と出ます。
- 「ビューアを複数起動しない」の設定（`bHtmlHelpIsSingle`）は、いまは使われません。ビューアは常に 1 つです。

[外部ヘルプ１](ext-help1.md)は、ファイルの拡張子で動きを変えます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp` の `CViewCommander::Command_EXTHTMLHELP`、`sakura_core/macro/CMacro.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/support/exthtmlhelp.rs`、`crates/matsu-ui/src/support/help_flow.rs`（`ext_html_help`、`MAX_KEYWORD_CHARS`）、`crates/matsu-ui/src/support/help.rs`（`ext_html_help`、`plan_html_help`、`perform`）、`crates/matsu-platform/src/help/mod.rs`（`keyword_lookup`）
- 試験: `crates/matsu-ui/src/support/help.rs` の `the_type_has_the_say_over_the_common_settings`、`exthelp1_looks_up_html_help_and_opens_other_files`
- 既定のキーとマクロ名: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F1` の行）、`crates/matsu-commands/src/sakura/macro_functions.rs`（`ExtHtmlHelp`）
