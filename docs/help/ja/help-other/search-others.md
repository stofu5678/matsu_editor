# 置換の実行、検索ボックス、そのほかの検索系

メニューにない、検索系の機能です。

| 機能名 | 表示名 | キー | マクロ（引数） | キーマクロ |
|---|---|---|---|---|
| `F_REPLACE` | 置換 | なし | `Replace(置換前, 置換後, フラグ)` | 記録する |
| `F_REPLACE_ALL` | すべて置換 | なし | `ReplaceAll(置換前, 置換後, フラグ)` | 記録する |
| `F_SEARCH_BOX` | 検索 (ボックス) | なし | なし | |

次の機能も、メニューにありません。「コマンド: 検索」の章に、ページがあります。

| 機能名 | ページ |
|---|---|
| `F_GREP` | Grep |
| `F_GREP_REPLACE` | Grep置換 |
| `F_JUMP` | 指定行へジャンプ |
| `F_OUTLINE_TOGGLE` | アウトライン解析 |
| `F_TAGJUMP_LIST` | キーワードを指定してタグジャンプ |
| `F_TAGJUMP_CLOSE` | タグジャンプ |
| `F_BOOKMARK_PATTERN` | ブックマーク |
| `F_DIFF` | DIFF差分表示 |

## 置換

`F_REPLACE`: 置換パネルの「置換」ボタンと同じ動きです。いまの検索の設定で、選択（なければキャレットの位置）にある一致を置き換え、次の一致を選びます。

- 次の一致がないときは、置き換えで終わり、「最後まで置換しました。」と箱で知らせます。
- まだ探す文字列がないときは、置換パネルを開きます。
- 正規表現が正しくないときは、箱で知らせて、何もしません。

引数をつけると、マクロとして、探す文字列、置き換える文字列、オプションを指定して動かせます。

- 引数なしなら、パネルの設定を使います。
- 探す文字列が空、または置き換える文字列がないときは、マクロはエラーで止まります。
- フラグの `0x40`（クリップボードの内容で置換する）は、まだ使えません。使うと、エラーになります。

`F_REPLACE_ALL`: 置換パネルの「すべて置換」ボタンと同じ動きです。一致のすべてを置き換えます。全体で元に戻す 1 回分（Undo の 1 段）になります。

- 引数の決まりは、`F_REPLACE` と同じです。
- 引数がなく、探す文字列もまだないときは、置換パネルを開きます。

パネルの項目、フラグの値は、「コマンド: 検索」の章の「置換...」のページを見てください。

## 検索 (ボックス)

`F_SEARCH_BOX`: ツールバーの検索ボックスに、フォーカスを移します。

- ツールバーが表示されていて、そこに検索ボックスがあるときだけ動きます。ないときは、何も起きません。
- マクロの名前はありません。キー割り当てやカスタムメニューから呼びます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Search.cpp` の `Command_REPLACE`、`Command_REPLACE_ALL`、`Command_SEARCH_BOX`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/search/replace.rs`、`replace_all.rs`、`search_box.rs`。`crates/matsu-ui/src/search/ops.rs`（`replace_next`、`replace_all`）、`crates/matsu-ui/src/search/macro_args.rs`（`with_search_args`）、`crates/matsu-ui/src/bars/toolbar.rs`（`has_search_box`、`focus_search_box`）
- 試験: `crates/matsu-ui/src/search/macro_args_tests.rs`、`crates/matsu-ui/src/bars/tests/search_box.rs`
- マクロ名と引数の型: `crates/matsu-commands/src/sakura/macro_functions.rs`
