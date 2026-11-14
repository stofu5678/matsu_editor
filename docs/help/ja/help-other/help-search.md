# ヘルプキーワード検索

選んでいる文字、または、キャレットの位置の言葉を、このヘルプの中から探します。

| 項目 | 内容 |
|---|---|
| メニュー | ヘルプ > ヘルプキーワード検索 |
| 機能名 | `F_HELP_SEARCH` |
| 既定のキー | `Alt+Shift+F1` |
| マクロ | `HelpSearch` |
| キーマクロに記録 | しない |

## 動き

1. 探す言葉を決めます。文字を選んでいれば、その最初の行の文字です。選んでいなければ、キャレットの位置の言葉です。前後の空白は除き、100 文字までです。この文字は、引用符で囲んで 1 つの語として探します（`NOT` や `path:` が入っていても、検索の書き方にはなりません）。
2. ブラウザーで、GitHub のコード検索を開きます。検索の範囲は、このリポジトリの `docs/help` です。
3. 探す言葉がないとき（空の文書の中など）は、[ヘルプ目次](help-contents.md)と同じものを開きます。

GitHub のコード検索は、GitHub にログインしていないと使えません。

## プログラムに付いているヘルプの検索

プログラムに付いているヘルプには、検索用のページがありません。ヘルプの検索の仕組み（Pagefind）は、ページが自分の索引のファイルを読みに行きますが、ブラウザーは、ファイルとして開いたページ（`file://`）にそれを許さないことが多いためです。MATSU Editor は、検索のためのサーバーをプログラムの中に持たないと決めています（ADR-0031）。

プログラムに付いているヘルプの中を探したいときは、1 ページにまとめた版（`print.html`）を開き、ブラウザーの検索（`Ctrl+F`。macOS は `Command+F`）を使ってください。

## サクラエディタとの違い

- サクラエディタは、自分のヘルプファイルの索引を、探す言葉を入れて開きます。MATSU Editor は、ウェブの検索を開きます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp` の `Command_HELP_SEARCH`、`sakura_core/func/CKeyBind.cpp`（`Alt+Shift+F1`）
- MATSU のコード: `crates/matsu-ui/src/support/help.rs`（`plan_search`）、`crates/matsu-ui/src/support/help_flow.rs`（`program_help`）、`crates/matsu-ui/src/ui_commands/support/help_search.rs`
- 試験: `crates/matsu-ui/src/support/help.rs` の `the_search_is_a_code_search_of_the_help_for_the_word`、`crates/matsu-ui/src/support/help_tests.rs` の `the_search_takes_the_selection_as_the_query`
- 決めたこと: ADR-0031
