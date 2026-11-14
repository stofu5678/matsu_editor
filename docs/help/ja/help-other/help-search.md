# ヘルプキーワード検索

ヘルプの索引にあたるページを、ブラウザで開きます。

- 機能名: `F_HELP_SEARCH`
- メニュー: 「ヘルプ」→「ヘルプキーワード検索」
- 既定のキー: `Alt+Shift+F1`（macOS では `Option+Shift+F1`）
- マクロ: `HelpSearch()`（引数なし。キーマクロには記録されません）

## 動き

既定のブラウザで、次のページを開きます。

```
https://github.com/stofu5678/matsu_editor/blob/main/docs/adr/README.md
```

- 決めたことの一覧（ADR の目次）です。
- キーワードを入力する欄は、マツエディタの側にはありません。探すときは、開いたページで探します。
- ネットにつながっていないと、開けません。

## サクラエディタとの違い

サクラエディタは、自分のヘルプのファイルを索引のページで開きます。マツエディタには索引がないので、いまは上のページで代用しています。ヘルプを GitHub Pages に出す計画があり（ADR-0031）、そのときは検索の入口が変わる見込みです。

文書の中の単語でヘルプを引く機能は、別にあります。[外部ヘルプ１](ext-help1.md)と[外部HTMLヘルプ](ext-html-help.md)を見てください。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp` の `CViewCommander::Command_HELP_SEARCH`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/support/help_search.rs`、`crates/matsu-ui/src/support/help.rs`（`INDEX_URL`）、`crates/matsu-ui/src/support/help_flow.rs`（`program_help`）
- 試験: `crates/matsu-ui/src/support/help.rs` の `the_urls_are_the_documents_of_the_project`
- 既定のキーとマクロ名: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F1` の行）、`crates/matsu-commands/src/sakura/macro_functions.rs`（`HelpSearch`）
- 計画: `docs/adr/0031-help.md`
