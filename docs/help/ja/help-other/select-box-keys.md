# 選択と矩形選択の移動、1行選択、モードの取り消し

メニューにない、選択系と矩形選択系の機能です。[カーソルの基本の移動とスクロール](cursor-scroll.md)の移動に、選択を広げる動きや矩形選択を足したものが中心です。

## 選択を広げる移動

「(選択)」で始まる機能です。動いた分だけ、選択を広げます。選択の始まりは、動く前のキャレットの位置です。すでに選択があるときは、その始まりを変えずに、キャレットの側を動かします。

| 機能名 | 表示名 | キー | マクロ（引数） |
|---|---|---|---|
| `F_UP_SEL` | (選択)カーソル上移動 | `Shift+↑` | `Up_Sel()` |
| `F_DOWN_SEL` | (選択)カーソル下移動 | `Shift+↓` | `Down_Sel()` |
| `F_LEFT_SEL` | (選択)カーソル左移動 | `Shift+←` | `Left_Sel()` |
| `F_RIGHT_SEL` | (選択)カーソル右移動 | `Shift+→` | `Right_Sel()` |
| `F_HalfPageUp_Sel` | (選択)半ページアップ | `Ctrl+Shift+PageUp` | `HalfPageUp_Sel(行数)` |
| `F_HalfPageDown_Sel` | (選択)半ページダウン | `Ctrl+Shift+PageDown` | `HalfPageDown_Sel(行数)` |

- 動く向きと量は、選択のない移動と同じです。上下を動いたときに表示桁を覚えることも同じです。
- 上下のキーを押しつづけると、1 回に動く行数が増えます（「共通設定」の「全般」の「スクロール」の「行数」。キーマクロの記録中は 1 行）。
- どれも、キーマクロに記録されます。

## 矩形選択を広げる移動

「(矩形選択)」で始まる機能です。矩形選択がまだないときは、キャレットの位置から矩形選択を始めて、そこから動いて矩形を広げます。

| 機能名 | 表示名 | キー | マクロ（引数） |
|---|---|---|---|
| `F_UP_BOX` | (矩形選択)カーソル上移動 | `Alt+↑` | `Up_BoxSel(数)` |
| `F_DOWN_BOX` | (矩形選択)カーソル下移動 | `Alt+↓` | `Down_BoxSel(数)` |
| `F_LEFT_BOX` | (矩形選択)カーソル左移動 | `Alt+←` | `Left_BoxSel(数)` |
| `F_RIGHT_BOX` | (矩形選択)カーソル右移動 | `Alt+→` | `Right_BoxSel(数)` |
| `F_HalfPageUp_BOX` | (矩形選択)半ページアップ | `Ctrl+Alt+PageUp` | `HalfPageUp_BoxSel(行数, 数)` |
| `F_HalfPageDown_BOX` | (矩形選択)半ページダウン | `Ctrl+Alt+PageDown` | `HalfPageDown_BoxSel(行数, 数)` |

- macOS では、`Alt` は `Option` です。
- 矩形の列は、表示桁で数えます。全角の文字と TAB も、左右の移動の 1 歩です。
- 先頭の行で上へ動いても、矩形選択は始まります。
- マクロの引数のうち、表の「数」（ロックの指定など）は、MATSU Editor はまだ読みません。半ページの「行数」だけは読みます。省略または 0 以下のときは、表示している行数の半分です。
- どれも、キーマクロに記録されます。
- 矩形選択の始め方と、矩形での編集は、「基本的な使い方」の章の矩形選択のページと、「コマンド: 編集」の章の矩形選択のページにあります。

## 1行選択

`F_SELECTLINE`（表示名「1行選択」）

- キー: トリプルクリック（`Shift` `Ctrl` `Alt` をどう押していても）。キーボードのキーは、ありません。
- マクロ: `SelectLine(数)`。キーマクロに記録されます。引数は使われません。
- キャレットのある行を、改行ごと選びます。折り返して何行にも見える行も、全体を選びます。
- 最後の行に改行がないときは、行の終わりまでを選びます。
- 最後の改行のあとの、空の最後の行にいるときは、選べるものがないので、選択を外します。

## 各種モードの取り消し

`F_CANCEL_MODE`（表示名「各種モードの取り消し」）

- キー: `Esc`
- マクロ: `CancelMode()`。キーマクロに記録されます。
- 選択、選択の固定（「範囲選択開始」の状態）、矩形選択を、やめます。キャレットは、いる場所のままです。
- 取り消すものがなくても、エラーにも、ビープ音にもなりません。
- 選択が何もない状態の「固定」と、何も囲んでいない矩形選択も、取り消します。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Cursor.cpp` の `Command_UP`、`Command_HalfPageUp` ほか、`sakura_core/cmd/CViewCommander.cpp` の `HandleCommand`（矩形選択を始める `Sub_BoxSelectLock`）、`sakura_core/cmd/CViewCommander_Select.cpp` の `Command_SELECTLINE`、`sakura_core/cmd/CViewCommander_ModeChange.cpp` の `Command_CANCEL_MODE`
- MATSU のコード: `crates/matsu-commands/src/commands/cursor/` の `cursor_up_sel.rs` ほか `*_sel.rs`。`crates/matsu-commands/src/commands/box_select/` の `cursor_up_box.rs`、`cursor_left_box.rs` ほか。`crates/matsu-ui/src/ui_commands/cursor/` の `half_page_up_sel.rs` ほか。`crates/matsu-ui/src/ui_commands/box_select/half_page_up_box.rs` ほか。`crates/matsu-ui/src/view/paging.rs`。`crates/matsu-commands/src/commands/select/select_line.rs`、`crates/matsu-commands/src/context/word.rs`（`select_line`）。`crates/matsu-commands/src/commands/mode/cancel_mode.rs`、`crates/matsu-commands/src/context/modes.rs`（`cancel_mode`）
- 試験: `cursor_up_sel.rs` の `moves`、`cursor_up_box.rs` の `moves_up_and_begins_the_box`、`cursor_left_box.rs` の `moves_left_and_begins_the_box`、`select_line.rs` の `selects_the_line_with_its_ending`、`cancel_mode.rs` の `drops_the_selection_and_keeps_the_caret`、`ends_a_box_selection`（どれも `crates/matsu-commands/src/commands/` の下）
- 既定のキー、マクロ名、キーマクロの記録: `crates/matsu-commands/src/sakura/key_bindings.rs`、`crates/matsu-commands/src/sakura/macro_functions.rs`
