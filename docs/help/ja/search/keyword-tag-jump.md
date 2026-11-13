# キーワードを指定してタグジャンプ

キーワードを入力して、タグファイルから該当するタグを探し、一覧から選んで移動します。

- メニュー: 検索 → キーワードを指定してタグジャンプ...
- 既定のキー: なし
- マクロ: `KeywordTagJump(キーワード)`（機能名 `F_TAGJUMP_KEYWORD`。キーワードは省略でき、入力欄の初期値になります）。同じ動きで、タグジャンプ一覧を開く機能名 `F_TAGJUMP_LIST` もあります（メニューにはありません）

ファイルに保存していない文書では、動きません（「タグジャンプできません」と出ます）。キーワードを渡さなかったときの入力欄の初期値は、直近に使ったキーワードです。

## パネル

[ダイレクトタグジャンプ](direct-tag-jump.md)の候補の一覧に、キーワードの入力欄と 2 つのチェックを足したパネルです。

| 項目 | 意味 |
|---|---|
| キーワード | 探す語。打つと、最後のキーから 0.7 秒後に一覧が更新される（開いた直後とチェックを替えた直後は 0.05 秒後） |
| 大文字小文字の同一視 | 大文字と小文字を区別しない（ASCII の文字だけ） |
| 文字列の途中にマッチ | キーワードがタグの名前の途中にあっても一致とする |

一覧の見方と、`Enter`、ダブルクリック、「ジャンプ」、`Esc` の動きは、ダイレクトタグジャンプと同じです。2 つのチェックと、最近のキーワードは覚えられます。タグファイルを次々に探すかは、共通設定「検索」の「キーワード指定のタグファイル検索」で選びます（既定は「次のファイルを検索する」。ほかに「完全一致でヒットしたら次のファイルを検索しない」もあります）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_TagJump.cpp` の `Command_TagJumpByTagsFileKeyword`、`sakura_core/dlg/CDlgTagJumpList.cpp`、`sakura_core/macro/CMacro.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/search/tagjump_keyword.rs`、`tagjump_list.rs`、`crates/matsu-ui/src/tags/list.rs`、`crates/matsu-ui/src/tags/mod.rs`、`crates/matsu-core/src/tag_file/mod.rs`、`crates/matsu-ui/src/common_settings/pages/search.rs`
