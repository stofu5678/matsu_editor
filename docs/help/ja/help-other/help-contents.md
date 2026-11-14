# ヘルプ目次

マツエディタのヘルプを、ブラウザで開きます。

- 機能名: `F_HELP_CONTENTS`
- メニュー: 「ヘルプ」→「ヘルプ目次」
- 既定のキー: `Alt+F1`（macOS では `Option+F1`）
- マクロ: `HelpContents()`（引数なし。キーマクロには記録されません）

## 動き

既定のブラウザで、次のページを開きます。

```
https://github.com/stofu5678/matsu_editor/tree/main/docs
```

- GitHub にある、リポジトリの `docs` フォルダの一覧です。
- マツエディタは、ヘルプのファイルを持っていません。ネットにつながっていないと、開けません。
- ウィンドウの中に、確認や設定の箱は出ません。ブラウザが開くだけです。

## サクラエディタとの違い

サクラエディタは、自分のヘルプのファイルを目次のページで開きます。マツエディタは、いまは上のページを開く代用です。ヘルプを GitHub Pages に出し、アプリからも開く計画があります（ADR-0031）。そのときは、このページの「動き」が変わります。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp` の `CViewCommander::Command_HELP_CONTENTS`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/support/help_contents.rs`、`crates/matsu-ui/src/support/help.rs`（`CONTENTS_URL`）、`crates/matsu-ui/src/support/help_flow.rs`（`program_help`）
- 試験: `crates/matsu-ui/src/support/help.rs` の `the_urls_are_the_documents_of_the_project`
- 既定のキーとマクロ名: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F1` の行）、`crates/matsu-commands/src/sakura/macro_functions.rs`（`HelpContents`）
- 計画: `docs/adr/0031-help.md`
