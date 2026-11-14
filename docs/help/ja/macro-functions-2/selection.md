# 選択の関数

単語・行・全体の選択と、選択を広げながら動く関数です。矩形選択は [矩形選択](box-selection.md) にあります。

「広げる移動」は、通常の移動と同じ動きをして、動く前の位置（選択の始点）から動いた先までを選びます。すでに選択があれば、始点はそのままです。選択ロック（`BeginSelect`）が入っているときは、通常の移動も同じように選択を広げます。

矩形選択の最中は、広げる移動が矩形を広げます。「行」は、折り返したときは、画面上の 1 行（表示行）です。

マツエディタの選択は、始点とキャレットの 2 つで持ちます。選択を作る関数は、キャレットを選択の終わり側に置きます。サクラは選択の範囲とキャレットを別々に持つので、キャレットの位置が違う関数があります（表の「MATSU での違い」）。

## 表

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `SelectWord` | なし | なし | キャレットの位置の単語を選ぶ。空白や記号の連なりも 1 語として選ぶ。行末や、末尾の空行など、単語がない場所では何も選ばず、前の選択だけを解除する（ビープ音は鳴らさない）。 | する | キャレットは単語の末尾に置く。 |
| `SelectAll` | なし | なし | 文書全体を選ぶ。選ぶ前の位置を移動履歴に残す。空の文書では何も選ばない。 | する | キャレットは文書の末尾に置く。 |
| `SelectLine` | 整数（読まない） | なし | キャレットのある 1 行を、改行ごと選ぶ。折り返した行も全体を選ぶ。最終行で改行がないときは、行末まで選ぶ。改行の後の空行では何も選ばず、選択を解除する。 | する | 引数は読まない（サクラも使わない）。キャレットは次の行頭に置く。 |
| `BeginSelect` | なし | なし | 選択ロックの入と切を切り替える。入のあいだ、通常の移動が選択を広げる。もう一度呼ぶと、ロックだけ切れて、選択は残る。選択を解除する操作（ロックなしの移動、文字入力など）でも、ロックは切れる。 | する | 記録なし |
| `Up_Sel` | なし | なし | 1 行上へ動いて、選択を広げる。 | する | 記録なし |
| `Down_Sel` | なし | なし | 1 行下へ動いて、選択を広げる。 | する | 記録なし |
| `Left_Sel` | なし | なし | 1 文字左へ動いて、選択を広げる。 | する | 記録なし |
| `Right_Sel` | なし | なし | 1 文字右へ動いて、選択を広げる。 | する | 記録なし |
| `Up2_Sel` | なし | なし | 2 行上へ動いて、選択を広げる。 | する | 記録なし |
| `Down2_Sel` | なし | なし | 2 行下へ動いて、選択を広げる。 | する | 記録なし |
| `WordLeft_Sel` | なし | なし | 手前の単語の先頭へ動いて、選択を広げる。 | する | 記録なし |
| `WordRight_Sel` | なし | なし | 次の単語の先頭へ動いて、選択を広げる。 | する | 記録なし |
| `GoLineTop_Sel` | 整数（行頭移動のフラグ） | なし | 表示行の行頭へ動いて、選択を広げる。フラグなし（0）では、インデントの次の文字へ先に止まり、もう一度で行頭へ行く。フラグは下の「フラグ」を見る。 | する | 記録なし |
| `GoLineEnd_Sel` | 整数（行末移動のフラグ） | なし | 表示行の行末へ動いて、選択を広げる。フラグは下の「フラグ」を見る。 | する | 記録なし |
| `HalfPageUp_Sel` | 整数（動く行数） | なし | 半ページ上へ動いて、選択を広げる。 | する | 引数の扱いに疑問がある（下の「ページ移動の引数」）。 |
| `HalfPageDown_Sel` | 整数（動く行数） | なし | 半ページ下へ動いて、選択を広げる。 | する | 同上 |
| `PageUp_Sel`、`1PageUp_Sel` | 整数（動く行数） | なし | 1 ページ上へ動いて、選択を広げる。ページの行数から 1 を引いた行だけ、キャレットと表示が動く。 | する | 同上 |
| `PageDown_Sel`、`1PageDown_Sel` | 整数（動く行数） | なし | 1 ページ下へ動いて、選択を広げる。 | する | 同上 |
| `GoFileTop_Sel` | なし | なし | 文書の先頭へ動いて、選択を広げる。 | する | 記録なし |
| `GoFileEnd_Sel` | なし | なし | 文書の末尾へ動いて、選択を広げる。 | する | 記録なし |
| `GoNextParagraph_Sel` | なし | なし | 次の段落へ動いて、選択を広げる。 | しない | 記録なし |
| `GoPrevParagraph_Sel` | なし | なし | 前の段落へ動いて、選択を広げる。 | しない | 記録なし |
| `GoModifyLineNext_Sel` | なし | なし | 次の変更行へ動いて、選択を広げる。移動履歴には残さない。 | する | 記録なし |
| `GoModifyLinePrev_Sel` | なし | なし | 前の変更行へ動いて、選択を広げる。移動履歴には残さない。 | する | 記録なし |

## フラグ

`GoLineTop_Sel` と `GoLineEnd_Sel` の引数は、次の値を足して作ります。ほかの桁は無視します。

| 値 | `GoLineTop_Sel` | `GoLineEnd_Sel` |
|---|---|---|
| 1 | インデントの次の文字で止まらず、行頭（行の左端）へ行く | 使わない |
| 4 | 選択を広げる（この関数は、もともと広げる） | 同じ |
| 8 | 折り返した行ではなく、元の行（改行から改行まで）を単位にする | 同じ。元の行の最後の表示行の末尾へ行く |

## ページ移動の引数

`HalfPageUp_Sel` などの引数は、動く行数です。0 以下や省略では、既定（半ページ、または 1 ページから 1 行引いた数）を使います。`int` を超える数は、範囲に切り詰めます。

コードの中で、2 つの道が食い違っています。

- コマンド本体は、この引数を読みます（試験があります）。
- スクリプトから引数を付けて呼ぶ道は、「引数をまだ使わない」というエラーを返す一覧に、この関数を入れています。

スクリプトからは、いまは引数なしで呼ぶのが確かです。この食い違いは、PR の「残っていること」に書きました。

## 根拠

- 表の元: `crates/matsu-commands/src/sakura/macro_functions.rs`（`SelectWord` から `GoModifyLinePrev_Sel`）
- サクラ: `sakura_core/cmd/CViewCommander.cpp`（`HandleCommand` の `F_SELECTLINE` ほか `_SEL` の行）、`sakura_core/cmd/CViewCommander_Select.cpp`（`Command_SELECTLINE`、`Command_SELECTWORD`、`Command_SELECTALL`、`Command_BEGIN_SELECT`）
- MATSU のコード（`crates/matsu-commands/src/commands/`）:
  - `select/select_word.rs`、`select/select_all.rs`、`select/select_line.rs`、`select/begin_sel.rs`
  - `cursor/cursor_up_sel.rs` ほか `cursor/*_sel.rs`
  - `context/motion.rs`（`move_line_top_flags`、`move_line_end_flags`、`LINE_TOP_ONLY`、`LINE_TOP_SELECT`、`LINE_TOP_LINE_UNIT`）
- MATSU のコード（`crates/matsu-ui/src/`）:
  - `ui_commands/cursor/half_page_up_sel.rs` ほか `ui_commands/cursor/*page*_sel.rs`
  - `ui_commands/select/modifyline_next_sel.rs`、`modifyline_prev_sel.rs`
  - `view/paging.rs`（`rows_arg`）
  - `macros/ui_calls.rs`（`IGNORES_ARGUMENTS`: スクリプトからの呼び出しが引数を断る場所）
- 試験: `crates/matsu-commands/src/commands/select/` の各ファイルの `tests`、`crates/matsu-ui/src/view/paging_tests.rs`（`the_page_commands_take_the_number_of_rows_of_the_macro`）
