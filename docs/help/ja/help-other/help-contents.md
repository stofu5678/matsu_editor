# ヘルプ目次

このヘルプの目次を開きます。

| 項目 | 内容 |
|---|---|
| メニュー | ヘルプ > ヘルプ目次 |
| 機能名 | `F_HELP_CONTENTS` |
| 既定のキー | `Alt+F1` |
| マクロ | `HelpContents` |
| キーマクロに記録 | しない |

## 動き

次の順で、最初に見つかったものを開きます。

1. プログラムに付いているヘルプ。プログラムの置き場所を基準に、次の場所の `index.html` を探します。

   | 配布物 | 場所 |
   |---|---|
   | Windows の zip とインストーラー | `matsu.exe` と同じフォルダーの `help` |
   | macOS の `.app` | `MATSU Editor.app/Contents/Resources/help` |
   | macOS と Linux の tar.gz | 実行ファイルと同じフォルダーの `help` |
   | Linux の `.deb` と `.rpm` | `/usr/share/matsu-editor/help`（実行ファイルの隣の `help` が、ここへのリンクになっている場合もあります） |

   上の表の順に探し、`index.html` がある最初のフォルダーを使います。ブラウザーで開きます。
2. 見つからなければ、ウェブ上のヘルプの目次（GitHub のリポジトリにある `docs/help/ja/SUMMARY.md`）を、ブラウザーで開きます。

開発中のビルドなど、ヘルプが付いていないプログラムでも、ウェブのヘルプを読めます。ウェブのヘルプは、リポジトリの main の内容なので、手元のプログラムの版と合わないことがあります。

`F1`（外部HTMLヘルプ）と `Ctrl+F1`（外部ヘルプ１）は、このコマンドではありません。利用者が設定した外部のヘルプを開きます（[共通設定: 支援](../common-settings/page-support.md)）。このヘルプは `Alt+F1` で開きます。

## ダイアログのヘルプボタン

ダイアログの「ヘルプ」ボタンは、そのダイアログの説明のページを開きます。プログラムに付いているヘルプにそのページがあればそれを、なければウェブのページを開きます。

| ダイアログ | 開くページ |
|---|---|
| 共通設定 | 表示しているページの説明（[共通設定](../common-settings/index.md)の各ページ）。シェルのページは説明がないので、共通設定の章の最初のページ |
| ファイルツリーの設定 | [ファイルツリー](../search/file-tree.md) |
| 文字色統一・背景色統一 | [タイプ別設定: 色](../type-settings-window/type-page-color.md) |

ほかのダイアログのヘルプボタンは、まだ働きません（無効のまま、またはありません）。

## ページの途中への移動

ページの見出しまで移る（`#見出し` を付けて開く）ことは、まだしていません。ページの先頭を開きます。OS とブラウザーによっては `#見出し` が落ちることが分かっておらず、確かめるまで付けないことにしています。

## サクラエディタとの違い

- サクラエディタは、自分のヘルプファイル（`sakura.chm`）を開きます。MATSU Editor のヘルプは、HTML のファイルの集まりで、ブラウザーで開きます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp` の `Command_HELP_CONTENTS`、`sakura_core/func/CKeyBind.cpp`（`Alt+F1`）、`sakura_core/func/Funccode.cpp` の `FuncID_To_HelpContextID`（機能からヘルプの項目を引く表）
- MATSU のコード: `crates/matsu-ui/src/support/help.rs`（`local_dir`、`plan_contents`、`PAGES`）、`crates/matsu-ui/src/ui_commands/support/help_contents.rs`
- 試験: `crates/matsu-ui/src/support/help.rs` の `the_local_help_is_looked_for_in_the_places_of_the_packages`、`the_contents_are_the_local_index_else_the_page_on_the_web`、`every_page_of_the_table_exists`
- 決めたこと: ADR-0031
