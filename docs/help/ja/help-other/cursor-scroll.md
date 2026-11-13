# カーソルの基本の移動とスクロール

メニューにない、カーソル移動系の機能です。矢印キー、ページ、スクロール、ホイールの機能が入っています。選択を広げる移動と矩形選択の移動は、[選択と矩形選択の移動、1行選択、モードの取り消し](select-box-keys.md)にあります。

## 一覧

| 機能名 | 表示名 | キー | マクロ（引数） | キーマクロ |
|---|---|---|---|---|
| `F_UP` | カーソル上移動 | `↑` | `Up()` | 記録する |
| `F_DOWN` | カーソル下移動 | `↓` | `Down()` | 記録する |
| `F_LEFT` | カーソル左移動 | `←` | `Left()` | 記録する |
| `F_RIGHT` | カーソル右移動 | `→` | `Right()` | 記録する |
| `F_HalfPageUp` | 半ページアップ | `Ctrl+PageUp` | `HalfPageUp(行数)` | 記録する |
| `F_HalfPageDown` | 半ページダウン | `Ctrl+PageDown` | `HalfPageDown(行数)` | 記録する |
| `F_CURLINETOP` | カーソル行をウィンドウ上部へ | なし | `CurLineTop()` | 記録する |
| `F_CURLINEBOTTOM` | カーソル行をウィンドウ下部へ | なし | `CurLineBottom()` | 記録する |
| `F_WndScrollDown` | テキストを１行下へスクロール | `Ctrl+↑` | `F_WndScrollDown()` | 記録しない |
| `F_WndScrollUp` | テキストを１行上へスクロール | `Ctrl+↓` | `F_WndScrollUp()` | 記録しない |
| `F_AUTOSCROLL` | オートスクロール | 中ボタンのクリック | なし | （マクロがないので、なし） |
| `F_WHEELUP` `F_WHEELDOWN` | ホイールアップ、ホイールダウン | ホイール（下の表） | `WheelUp(量)` `WheelDown(量)` | 記録しない |
| `F_WHEELLEFT` `F_WHEELRIGHT` | ホイール左、ホイール右 | 横ホイール（下の表） | `WheelLeft(量)` `WheelRight(量)` | 記録しない |
| `F_WHEELPAGEUP` `F_WHEELPAGEDOWN` | ホイールページアップ、ホイールページダウン | なし | `WheelPageUp(量)` `WheelPageDown(量)` | 記録しない |
| `F_WHEELPAGELEFT` `F_WHEELPAGERIGHT` | ホイールページ左、ホイールページ右 | なし | `WheelPageLeft(量)` `WheelPageRight(量)` | 記録しない |

- キーは、macOS では `Alt` が `Option`、`Ctrl` が `Control` です。
- `F_WndScrollDown` と `F_WndScrollUp` のマクロの名前には、`F_` が付いています（サクラエディタの表のままです）。
- 「記録する」「記録しない」は、キーマクロに記録されるかどうかです（キーマクロは、「マクロの使い方」の章を見てください）。

## 矢印キー

- `F_UP` `F_DOWN`: 画面の 1 行ぶん、上（下）へ動きます。動く前に覚えていた表示桁へ行きます。短い行をまたいでも、元の桁を覚えています。ほかの移動をすると、その桁は忘れます。先頭の行で上へ、最後の行で下へは、動きません。
- `F_LEFT` `F_RIGHT`: 1 文字ぶん、左（右）へ動きます。行の端では、前（次）の行へ移ります。選択があるときは、選択を外して、左なら選択の始まり、右なら選択の終わりへ動きます。
- 「範囲選択開始」や「矩形範囲選択開始」で選択を固定しているときは、どれも選択を広げる動きになります。
- 上下のキーを押しつづけたとき（キーの自動繰り返し）は、1 回に 3 行ずつ動きます。この行数は、「共通設定」→「全般」の「スクロール」の「行数」（`matsu.ini` の `nRepeatedScrollLineNum`）で変えられます。キーマクロを記録している間は、繰り返しても 1 行です。

## 半ページ

- キャレットが、表示している行の半分の行数だけ、上（下）へ動きます。表示は、キャレットが見えるのに必要なぶんだけ、あとからついていきます。
- マクロの引数は、動く行数です。省略したとき、または 0 以下のときは、表示している行の半分です。

## カーソル行をウィンドウ上部へ、下部へ

キャレットは動かさず、表示だけをスクロールして、キャレットのある行を、窓の一番上（下）に置きます。

- 「下部へ」は、全体が見えている最後の行に置きます。サクラエディタは、その 1 行下（窓の外）に置く動きです。

## テキストを 1 行スクロール

- `Ctrl+↑`（`F_WndScrollDown`）は表示を 1 行ぶん下へ、`Ctrl+↓`（`F_WndScrollUp`）は 1 行ぶん上へ動かします。
- キャレットは、選択がないとき、窓の端に近づきすぎないように、一緒に動きます。近さの目安は、表示している行数の 20 分の 1（1 行より小さくならない）です。選択があるときは、キャレットは動きません。

## オートスクロール

中ボタンを押して離すと、ポインタを動かした向きと距離で、自動でスクロールします。

- 開始した位置から離れるほど速くなります。200 ミリ秒ごとに、距離を 8 で割った数から 1 を引いた行数（列数）だけ動きます。
- 文書が窓より長いときは縦に、広いときは横に動きます。どちらにも動けないときは、始まりません。
- 次のどれかで終わります: もう一度の中ボタン、左右のボタンのクリック、キー、ほかの機能、ウィンドウがフォーカスを失うこと。
- キーやメニューからこの機能を動かすと、ポインタの位置から始まります。動いている間に動かすと、終わります。
- 開始位置に出る小さな窓と、向きを示すポインタの形は、まだありません。

## ホイール

| 機能 | 動き |
|---|---|
| `F_WHEELUP` `F_WHEELDOWN` | 上（下）へ、1 目盛りで 3 行 |
| `F_WHEELLEFT` `F_WHEELRIGHT` | 左（右）へ、1 目盛りで 3 桁分の幅 |
| `F_WHEELPAGEUP` `F_WHEELPAGEDOWN` | 上（下）へ、1 目盛りで 1 ページ（表示している行数より 1 行少ない） |
| `F_WHEELPAGELEFT` `F_WHEELPAGERIGHT` | 左（右）へ、1 目盛りで 1 ページ（表示している桁数より 1 桁少ない） |

キーの表（既定）で、ホイールに割り当てられているものは次のとおりです。

| 操作 | 機能 |
|---|---|
| ホイールを上へ | `F_WHEELUP`。ただし `Ctrl` を押しているときは `F_SETFONTSIZEUP`（[設定まわりのメニューにない機能](settings-others.md)） |
| ホイールを下へ | `F_WHEELDOWN`。ただし `Ctrl` を押しているときは `F_SETFONTSIZEDOWN` |
| 横ホイール | `F_WHEELLEFT` `F_WHEELRIGHT`（どの修飾キーでも） |

- ページ単位の 4 つの機能は、既定では、どの操作にも割り当てられていません。
- 「共通設定」の「全般」で、キーやボタンを押しながらホイールを回したとき、ページ単位や横に動かすように選べます（「基本的な使い方」の章の、マウスのページを見てください）。
- 1 目盛りで動く行数の既定は 3 行です。Windows では、OS の設定の値に従います。
- トラックパッドなど、細かく動く入力では、通常のホイール（`F_WHEELUP` など）は、動いた距離のとおりになめらかにスクロールします。ページ単位のときは、目盛りに換算します。
- マクロの `WheelUp(量)` などの量は、ホイールの回転で、1 目盛りが 120 です。省略したときと 0 のときは、1 目盛りです。向きは機能が決めるので、量の大きさだけを使います。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Cursor.cpp` の `Command_UP`、`Command_DOWN`、`Command_LEFT`、`Command_RIGHT`、`Command_HalfPageUp`、`Command_HalfPageDown`、`Command_CURLINETOP`、`Command_CURLINEBOTTOM`、`Command_WndScrollDown`、`Command_WndScrollUp`、`Command_AUTOSCROLL`、`Command_WHEELUP` など。`sakura_core/view/CEditView_Mouse.cpp`（`OnMOUSEWHEEL2`、`AutoScrollEnter`）
- MATSU のコード: `crates/matsu-commands/src/commands/cursor/` の `cursor_up.rs`、`cursor_down.rs`、`cursor_left.rs`、`cursor_right.rs`。`crates/matsu-ui/src/ui_commands/cursor/` の `half_page_up.rs`、`half_page_down.rs`、`cur_line_top.rs`、`cur_line_bottom.rs`、`wnd_scroll_down.rs`、`wnd_scroll_up.rs`、`autoscroll.rs`、`wheelup.rs` ほか。`crates/matsu-ui/src/view/paging.rs`（`half_page_by`、`scroll_window_by`、`key_repeat_count`、`handle_wheel`、`scroll_by_wheel_function`）、`crates/matsu-ui/src/view/autoscroll.rs`、`crates/matsu-ui/src/common_settings/pages/general.rs`（`ScrollOptions`）
- 試験: `crates/matsu-commands/src/commands/cursor/cursor_up.rs` の `remembers_the_column_over_short_rows`、`cursor_left.rs`、`cursor_right.rs` の `moves`、`crates/matsu-ui/src/view/paging_tests.rs`
- 既定のキー、マクロ名、キーマクロの記録: `crates/matsu-commands/src/sakura/key_bindings.rs`、`crates/matsu-commands/src/sakura/macro_functions.rs`
