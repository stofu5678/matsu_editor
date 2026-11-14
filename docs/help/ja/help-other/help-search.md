# ヘルプキーワード検索

ヘルプの中から、言葉を探すためのページを、ブラウザで開きます。

- 機能名: `F_HELP_SEARCH`
- メニュー: 「ヘルプ」→「ヘルプキーワード検索」
- 既定のキー: `Alt+Shift+F1`（macOS では `Option+Shift+F1`）
- マクロ: `HelpSearch()`（引数なし。キーマクロには記録されません）

## 動き

開くものは、プログラムに付いているヘルプがあるかどうかで決まります（探す場所は[ヘルプ目次](help-contents.md)と同じです）。

- **ヘルプがあるとき**: ヘルプ全体を 1 ページにまとめた版（`search.html`。`print.html` と同じ内容で、開いたときに印刷の画面が出ないもの）を開きます。ステータスバーに「ブラウザーの検索で探してください」と出るので、`Ctrl+F`（macOS は `Command+F`）で探します。ログインは要りません。
- **ヘルプがないとき**（`search.html` のない古い配布物を含む）: GitHub のコード検索を、このリポジトリの `docs/help` に絞って開きます。探す言葉は、選んでいる文字、なければキャレットの位置の言葉です（最初の行の 100 文字まで。1 つの語として探します）。言葉がなければ、ウェブの目次を開きます。**GitHub にログインしていないと使えません。** ヘルプを GitHub Pages に出すまでは、このままです。

ヘルプの検索ページ（Pagefind）は使いません。ブラウザは、ファイルとして開いたページ（`file://`）が自分の索引を読むことを許さないと見られるためです（確かめてはいません）。マツエディタは、検索のためのサーバーを持たないと決めています（ADR-0031）。

## サクラエディタとの違い

サクラエディタは、自分のヘルプのファイルを索引のページで開きます。マツエディタには索引がなく、上のとおり、1 ページ版かウェブの検索で代わりにしています。

文書の中の単語でヘルプを引く機能は、別にあります。[外部ヘルプ１](ext-help1.md)と[外部HTMLヘルプ](ext-html-help.md)を見てください。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp` の `CViewCommander::Command_HELP_SEARCH`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/support/help_search.rs`、`crates/matsu-ui/src/support/help.rs`（`plan_search`）、`crates/matsu-ui/src/support/help_flow.rs`（`program_help`）
- 試験: `crates/matsu-ui/src/support/help.rs` の `the_search_opens_the_one_page_book_when_the_program_has_the_help`、`the_search_is_a_code_search_of_the_help_for_the_word`
- 既定のキーとマクロ名: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F1` の行）、`crates/matsu-commands/src/sakura/macro_functions.rs`（`HelpSearch`）
- 計画: `docs/adr/0031-help.md`
