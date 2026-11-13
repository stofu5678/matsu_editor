# 外部ヘルプ１

利用者が設定したヘルプのファイルを、カーソルのある単語で開きます。

- 機能名: `F_EXTHELP1`
- メニュー: 「ヘルプ」→「外部ヘルプ１」
- 既定のキー: `Ctrl+F1`
- マクロ: `ExtHelp1()`（引数なし。キーマクロには記録されません）

## 使うファイル

次の順に、最初に空でないものを使います。

1. 文書のタイプの「タイプ別設定」→「支援」の「外部ヘルプの設定」
2. 「共通設定」→「支援」の「外部ヘルプの設定」（`matsu.ini` の `szExtHelp`）

どちらも空のときは、「外部ヘルプ１が設定されていません。今すぐ設定しますか?」と聞きます。

- 「はい(Y)」: 「共通設定」を「支援」のページで開き、この機能は終わります。ファイルを設定したあとで、もう一度この機能を動かしてください。
- 「いいえ(N)」と「キャンセル」: 何もしません。

ファイルの名前が相対パスのときは、`matsu.ini` のあるフォルダを先に探します。そこになければ、マツエディタの実行ファイルのあるフォルダを探します。どちらにもなければ、`matsu.ini` のあるフォルダのファイルとして扱います。

## 開き方

探す言葉は、選択範囲があれば、その最初の行です。なければ、カーソル位置の単語です。どちらもなければ、言葉なしで開きます。

| ファイルの拡張子 | 動き |
|---|---|
| `.chm` `.chi` `.col`（大文字と小文字は区別しない） | HTML ヘルプとして、言葉を引いて開く |
| それ以外（`.hlp` など） | OS が割り当てているアプリで、ファイルを開く。言葉は渡さない |

HTML ヘルプの言葉引きができるのは、Windows だけです。Windows 以外では、ファイルを OS のアプリで開き、ステータスバーに「この環境ではヘルプをキーワードで検索できないため、ファイルを開きました」と出ます。Windows でも言葉引きに失敗したときは、同じように、ファイルを開いてこのメッセージを出します。

「ビューアを複数起動しない」の設定（`bHtmlHelpIsSingle`）は、いまは使われません。マツエディタのウィンドウは 1 つのプロセスなので、ヘルプのビューアも 1 つです。

## 外部HTMLヘルプとの違い

[外部HTMLヘルプ](ext-html-help.md)は、拡張子を見ずに、いつも HTML ヘルプとして言葉を引きます。既定のキーは `F1` です。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp` の `CViewCommander::Command_EXTHELP1`、`sakura_core/env/CHelpManager.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/support/exthelp1.rs`、`crates/matsu-ui/src/support/help_flow.rs`（`ext_help1`、`ask_to_set_help`）、`crates/matsu-ui/src/support/help.rs`（`ext_help`、`inidir_or_exedir`、`plan_ext_help`、`perform`）、`crates/matsu-platform/src/help/mod.rs`（`is_html_help_file`、`keyword_lookup`）、`crates/matsu-ui/src/common_settings/pages/support.rs`、`crates/matsu-ui/src/search/state.rs`（`seed_text`）
- 試験: `crates/matsu-ui/src/support/help.rs` の `the_type_has_the_say_over_the_common_settings`、`a_relative_path_is_found_under_the_settings_folder_then_the_program_folder`、`exthelp1_looks_up_html_help_and_opens_other_files`、`crates/matsu-platform/src/help/mod.rs` の `html_help_files_are_told_by_the_extension`
- 既定のキーとマクロ名: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F1` の行）、`crates/matsu-commands/src/sakura/macro_functions.rs`（`ExtHelp1`）
