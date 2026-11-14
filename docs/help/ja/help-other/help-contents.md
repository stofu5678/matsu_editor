# ヘルプ目次

マツエディタのヘルプを、ブラウザで開きます。

- 機能名: `F_HELP_CONTENTS`
- メニュー: 「ヘルプ」→「ヘルプ目次」
- 既定のキー: `Alt+F1`（macOS では `Option+F1`）
- マクロ: `HelpContents()`（引数なし。キーマクロには記録されません）

## 動き

ヘルプの目次を、既定のブラウザで開きます。開くものは、次の順で決まります。

1. プログラムに付いているヘルプ（`index.html`）。実行ファイルの置き場所を基準に、次の順で探し、最初に見つかったものを使います。

   | 探す場所 | 使う配布物 |
   |---|---|
   | 実行ファイルと同じフォルダの `help` | Windows の zip とインストーラー、macOS と Linux の tar.gz。Linux の `.deb` と `.rpm` では、ここが `/usr/share/matsu-editor/help` へのリンクです |
   | 実行ファイルの 1 つ上の `Resources/help` | macOS の `.app` |
   | 実行ファイルの 1 つ上の `share/matsu-editor/help` | Linux の `.deb` と `.rpm` |

2. 見つからなければ、ウェブ上のヘルプの目次を開きます。

   ```
   https://github.com/stofu5678/matsu_editor/blob/main/docs/help/ja/SUMMARY.md
   ```

   GitHub にあるソースの目次で、ネットにつながっていないと開けません。リポジトリの main の内容なので、手元の版と合わないことがあります。

- ウィンドウの中に、確認や設定の箱は出ません。ブラウザが開くだけです。
- `F1` と `Ctrl+F1` は、これではなく、設定した外部のヘルプを開きます（[外部HTMLヘルプ](ext-html-help.md)、[外部ヘルプ１](ext-help1.md)）。

## ダイアログのヘルプボタン

ダイアログの「ヘルプ」ボタンは、上と同じ探し方で、そのダイアログのページを開きます。

| ダイアログ | 開くページ |
|---|---|
| 共通設定 | 表示しているページの説明。タイプ別設定のシートでは、タイプ別設定の章の最初のページ |
| ファイルツリーの設定 | [ファイルツリー](../search/file-tree.md) |
| 文字色統一・背景色統一 | [タイプ別設定: 色](../type-settings-window/type-page-color.md) |

ほかのダイアログのボタンは、まだ働きません。ページの途中の見出しまでは移らず、ページの先頭を開きます（`#見出し` がブラウザまで届くか、確かめていないため）。

## サクラエディタとの違い

サクラエディタは、自分のヘルプのファイル（`sakura.chm`）を目次のページで開きます。マツエディタのヘルプは、HTML のファイルの集まりで、ブラウザで開きます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp` の `CViewCommander::Command_HELP_CONTENTS`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/support/help_contents.rs`、`crates/matsu-ui/src/support/help.rs`（`local_dir`、`plan_contents`、`PAGES`）、`crates/matsu-ui/src/support/help_flow.rs`（`program_help`）
- 試験: `crates/matsu-ui/src/support/help.rs` の `the_local_help_is_looked_for_in_the_places_of_the_packages`、`the_contents_are_the_local_index_else_the_page_on_the_web`、`every_page_of_the_table_exists`
- 既定のキーとマクロ名: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F1` の行）、`crates/matsu-commands/src/sakura/macro_functions.rs`（`HelpContents`）
- 計画: `docs/adr/0031-help.md`
