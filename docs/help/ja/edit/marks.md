# 関数リストマークと変更行への移動

アウトライン解析の結果の行や、変更した行をたどる 4 つの機能です。「編集」→「移動」にあります。

| 機能名 | メニュー | キー | マクロ |
|---|---|---|---|
| `F_FUNCLIST_NEXT` | 次の関数リストマーク | なし | `FuncListNext()` |
| `F_FUNCLIST_PREV` | 前の関数リストマーク | なし | `FuncListPrev()` |
| `F_MODIFYLINE_NEXT` | 次の変更行へ | なし | `GoModifyLineNext()` |
| `F_MODIFYLINE_PREV` | 前の変更行へ | なし | `GoModifyLinePrev()` |

## 関数リストマーク

- 最後に行ったアウトライン解析で見つかった項目（関数など）の行に、印が付きます。「次の関数リストマーク」「前の関数リストマーク」は、その印のある次（前）の行の先頭へ動きます。
- 動きは、ふつうの移動です。「範囲選択開始」で選択を固定している間は、選択を広げます。
- 検索の設定「先頭（末尾）から再検索」が入っていると、見つからなかったときに、文書の反対の端から探し続けます。このとき、カーソルのある行が見つかることもあります。

## 変更行

- 変更した行のかたまりの、次（前）の端へ動きます。
- 反対の端から探し続けたときは、ステータスバーにそう出ます。見つからなかったときも、ステータスバーに出ます。共通設定の「見つからないときにメッセージを表示」が入っていれば、メッセージボックスも出ます。
- 移動履歴には、登録されません。
- 選択を広げながら動く版が、「選択」の副メニューにあります（[選択の移動](selection-moves.md)）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Bookmark.cpp` の `Command_FUNCLIST_NEXT`、`Command_FUNCLIST_PREV`、`sakura_core/cmd/CViewCommander_Cursor.cpp` の `Command_MODIFYLINE_NEXT`、`Command_MODIFYLINE_PREV`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/search/funclist_next.rs`、`funclist_prev.rs`、`crates/matsu-commands/src/context/funclist.rs`、`crates/matsu-ui/src/ui_commands/cursor/modifyline_next.rs`、`modifyline_prev.rs`、`crates/matsu-ui/src/search/modified_lines.rs`
