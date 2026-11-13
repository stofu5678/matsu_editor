# マクロの関数（4）: 桁・行の換算と表示の状態

折り返しの行と文書の行の換算、文字列の幅、表示領域の大きさ、タブ幅・折り返し桁・タブキーの空白入力の読み書き、描画スイッチ、ステータスバーの表示を扱う関数の表です。共通の決まりは[この章の最初](index.md)にあります。

- 「キーマクロに記録」は、どの行も「しない」です。
- 言葉の決まりは、[文書と選択の状態](document.md)の最初と同じです。行は「文書の行」と「折り返しの行」、桁は「表示桁」と「文字の位置」を使い分けます。数はどれも 1 から数えます。
- 折り返さない設定のときは、折り返しの行と文書の行が同じです。

## 折り返しの行と文書の行の換算

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `LayoutToLogicLineNum` | 1 折り返しの行（数） | 数 | その折り返しの行がある、文書の行の番号 | しない | 最後の折り返しの行の次の行を渡すと、最後の文書の行の次の番号を返す。ほかの範囲外の行はエラー |
| `LogicToLayoutLineNum` | 1 文書の行（数）、2 文字の位置（数） | 数 | 文書の行の、その文字の位置がある折り返しの行の番号 | しない | 最後の行より後ろの行を渡すと、最後の折り返しの行の次の番号を返す。文字の位置は検査しない |
| `LineColumnToIndex` | 1 折り返しの行（数）、2 表示桁（数） | 数 | 折り返しの行と表示桁にある文字の、文書の行の中での文字の位置 | しない | 表示桁が行の終わりを越えるときは、その段の文字の終わりの位置を返す（サクラエディタは、改行を越えた位置を返す）。表示桁は検査しない |
| `LineIndexToColumn` | 1 文書の行（数）、2 文字の位置（数） | 数 | 文書の行の、その文字の位置にある表示桁 | しない | 文字の位置が行の終わりを越えるときは、文字の終わりの桁を返す。その行に改行があるときは、改行の印の分の 1 を足した桁を返す（改行のない最後の行には足さない）。CR LF の中の位置は区別しない |

- 4 つとも、行は 1 以上でなければいけません。0 や負の数は、エラーです（`LayoutToLogicLineNum` は、範囲外のとき上の表のとおり）。
- 文字の位置は UTF-16 の単位で数えます。BMP の外の文字（絵文字など）は、位置は 2、表示桁も 2 進みます。

## 文字列の幅

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `GetStrWidth` | 1 文字列、2 開始桁?（数。省くと 1） | 数 | 文字列の幅（セルの数）。開始桁は、タブの幅を決めるために使う | しない | 末尾の CR LF は、CR だけとして数える |
| `GetStrLayoutLength` | 1 文字列、2 開始桁?（数。省くと 1） | 数 | 文書の 1 行として数えたときの幅。末尾の改行（CR LF、CR、LF、NEL、LS、PS）は、種類にかかわらず 1 セル | しない | 違いなし |
| `GetDefaultCharLength` | なし | 数 | 半角 1 文字の幅 | しない | 常に 1。サクラエディタはフォントの幅（ピクセル）で答える。MATSU Editor は、セル単位で並べるため、1 |

- タブは、開始桁から次のタブ位置までの幅になります。タブ幅は、そのウィンドウのいまの設定です。
- 全角の文字は 2、半角は 1 です。

## 表示領域

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `GetViewLines` | なし | 数 | 文字を表示する領域の段数（1 以上） | しない | ウィンドウがないときは、エラー |
| `GetViewColumns` | なし | 数 | 文字を表示する領域の桁数（1 以上） | しない | 同上 |
| `GetViewTop` | なし | 数 | 領域の先頭にある折り返しの行の番号 | しない | 同上 |
| `IsShownStatus` | なし | 数 | ステータスバーを出していれば 1、隠していれば 0 | しない | 違いなし |

## タブ幅・折り返し桁・タブキーの空白

3 つとも、引数に新しい値を渡すと、このウィンドウだけの設定が変わります。変えた値は保存しません。値が範囲外のときは、変えずに、いまの値だけを返します。

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `ChangeTabWidth` | 1 幅（数） | 数 | 変える前のタブ幅を返す。1 から 64 を渡すと、その幅に変える。0 などは、読むだけ | しない | 文として `.mac` に書くと、返った値は捨てて、変えるだけ |
| `ChangeWrapColm` `ChangeWrapColumn` | 1 桁（数） | 数 | 変える前の折り返し桁を返す。10 から 10240 を渡すと、その桁で折り返す設定にする（折り返し方は「指定桁」になる）。範囲外は読むだけ | しない | 2 つの名前は同じ関数。同じく `.mac` の文として書ける |
| `ChangeInsertSpace` | 1 値（数） | 数 | 変える前の「Tab キーで空白を入れる」設定を返す（1 は入れる、0 は入れない）。0 を渡すと切り、1 を渡すと入れる設定にする。それ以外は読むだけ | しない | 値は、下位 16 ビットだけを見る（65536 は 0、65537 は 1。サクラエディタと同じ）。文書だけの設定で、保存しない。`.mac` の文として書ける |

## 描画スイッチ

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `SetDrawSwitch` | 1 スイッチ（数） | 数 | スイッチを入れる（0 以外）か切る（0）。返すのは変える前の状態（1 か 0） | しない | スイッチは、`GetDrawSwitch` に答えるためだけに覚える。文字の描画は、スイッチを切っても止まらない（マクロの間は、描画が割り込まないため）。`.mac` の文として書くときは、数の下位 16 ビットだけを見る |
| `GetDrawSwitch` | なし | 数 | スイッチが入っていれば 1（初期値）、マクロが切っていれば 0 | しない | 違いなし |

## 根拠

- サクラのソース: `sakura_core/macro/CMacro.cpp` の `CMacro::HandleFunction`（`F_LAYOUTTOLOGICLINENUM`、`F_GETSTRWIDTH`、`F_CHGTABWIDTH`、`F_CHGINSSPACE`、`F_SETDRAWSWITCH` など）、`sakura_core/doc/layout/CLayoutMgr.cpp` の `LogicToLayout`、`LayoutToLogicEx`
- MATSU のコード: `crates/matsu-plugin/src/functions/layout.rs`、`crates/matsu-plugin/src/functions/misc.rs`（`set_draw_switch`、`get_draw_switch`、`is_shown_status`）、`crates/matsu-ui/src/macros/host.rs`（`finish_host_env`、`set_tab_width`、`set_wrap_columns`）、`crates/matsu-ui/src/macros/statements.rs`（`.mac` の文）
- 試験: `crates/matsu-plugin/src/functions/layout.rs` の `indexes_count_utf16_and_columns_count_cells`、`an_index_after_the_line_gives_the_line_ending_cell`、`the_width_of_a_string_follows_tabs_and_full_width_characters`、`the_layout_length_counts_a_final_line_ending_as_one_cell`、`the_tab_width_is_read_and_changed_by_request`、`the_wrap_column_is_read_and_changed_by_request`、`the_insert_space_setting_is_read_and_changed_by_request`、`crates/matsu-plugin/src/functions/misc.rs` の `the_draw_switch_answers_the_old_state`
