# 移動履歴と検索開始位置

行き来した位置の履歴をたどる 3 つの機能と、検索を始めた位置に戻る機能です。いずれも「編集」→「移動」にあります。

| 機能名 | メニュー | キー | マクロ |
|---|---|---|---|
| `F_JUMPHIST_PREV` | 移動履歴: 前へ | `Alt+P` | `MoveHistPrev()` |
| `F_JUMPHIST_NEXT` | 移動履歴: 次へ | `Alt+N` | `MoveHistNext()` |
| `F_JUMPHIST_SET` | 現在位置を移動履歴に登録 | なし | `MoveHistSet()` |
| `F_JUMP_SRCHSTARTPOS` | 検索開始位置へ戻る | `Ctrl+Shift+F3` | `SearchStartPos()` |

## 移動履歴

- 「現在位置を移動履歴に登録」は、今のカーソル位置を履歴に足します。
- 「移動履歴: 前へ」は、履歴を 1 つ前の位置へ戻ります。今の位置より後ろに履歴がないときは、先に今の位置を履歴に足します。そのため、あとで「次へ」で戻ってこられます。
- 「移動履歴: 次へ」は、履歴を 1 つ先へ進みます。進む先がなければ、何も起きません。
- 履歴は、文書を編集しても更新されません。位置が文書の外になったときは、いちばん近い位置へ動きます。
- 「範囲選択開始」で選択を固定している間は、動いた先まで選択を広げます。
- 「指定行へジャンプ」は、動く前に位置を自動で履歴へ足します（[指定行へジャンプ](jump.md)）。

## 検索開始位置へ戻る

検索パネルで最後に検索を始めた位置に、カーソルを戻します。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Cursor.cpp` の `Command_JUMPHIST_PREV`、`Command_JUMPHIST_NEXT`、`Command_JUMPHIST_SET`、`sakura_core/cmd/CViewCommander_Bookmark.cpp` の `Command_JUMP_SRCHSTARTPOS`
- MATSU のコード: `crates/matsu-commands/src/commands/cursor/jumphist_prev.rs`、`jumphist_next.rs`、`jumphist_set.rs`、`crates/matsu-commands/src/context/paragraph.rs`（履歴の動き）、`crates/matsu-ui/src/ui_commands/search/jump_srchstartpos.rs`、`crates/matsu-ui/src/search/ops.rs`
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`P`、`N`、`F3` の行）
