# ヘルプキーワード検索

ヘルプの中から、言葉を探すためのページを、ブラウザで開きます。

- 機能名: `F_HELP_SEARCH`
- メニュー: 「ヘルプ」→「ヘルプキーワード検索」
- 既定のキー: `Alt+Shift+F1`（macOS では `Option+Shift+F1`）
- マクロ: `HelpSearch()`（引数なし。キーマクロには記録されません）

## 動き

開くものは、プログラムに付いているヘルプがあるかどうかで決まります（探す場所は[ヘルプ目次](help-contents.md)と同じです）。

- **ヘルプがあるとき**: ヘルプ全体を 1 ページにまとめた版（`search.html`。`print.html` と同じ内容で、開いたときに印刷の画面が出ないもの）を開きます。ステータスバーに「ブラウザーの検索で探してください」と出るので、`Ctrl+F`（macOS は `Command+F`）で探します。ログインは要りません。
- **ヘルプがないとき**（`search.html` のない古い配布物を含む）: GitHub Pages に公開しているヘルプの検索ページ（<https://stofu5678.github.io/matsu_editor/find.html>）を開きます。選んでいる文字（選んでいなければキャレットの位置の言葉）が検索の箱に入り、開いたとたんに検索します（例: `find.html?q=grep`）。ステータスバーの案内は出ません。ネットにつながっていないと開けません。ログインは要りません。言葉は、開く URL の一部として GitHub Pages に渡ります（ブラウザがそのページを開くため）。それ以外には、どこにも送りません。

手元のヘルプでは、Pagefind の検索ページを使いません。ブラウザは、ファイルとして開いたページ（`file://`）が自分の索引を読むことを許さないと見られるためです（確かめてはいません）。マツエディタは、検索のためのサーバーを持たないと決めています（ADR-0031）。

## サクラエディタとの違い

サクラエディタは、自分のヘルプのファイルを索引のページで開きます。マツエディタには索引がなく、上のとおり、手元の 1 ページ版か、ウェブの検索ページで代わりにしています。

文書の中の単語でヘルプを引く機能は、別にあります。[外部ヘルプ１](ext-help1.md)と[外部HTMLヘルプ](ext-html-help.md)を見てください。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp` の `CViewCommander::Command_HELP_SEARCH`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/support/help_search.rs`、`crates/matsu-ui/src/support/help.rs`（`plan_search`）、`crates/matsu-ui/src/support/help_flow.rs`（`program_help`）
- 試験: `crates/matsu-ui/src/support/help.rs` の `the_search_opens_the_one_page_book_when_the_program_has_the_help`、`the_search_without_the_help_is_the_search_page_on_github_pages_at_the_word`
- 既定のキーとマクロ名: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F1` の行）、`crates/matsu-commands/src/sakura/macro_functions.rs`（`HelpSearch`）
- 計画: `docs/adr/0031-help.md`
