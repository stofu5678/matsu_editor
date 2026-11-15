# ドラッグ&ドロップ

マツエディタのドラッグ&ドロップは、2 つあります。窓の中で選んだ文字列を動かすことと、エクスプローラーなどからファイルを落として開くことです。

## 選んだ文字列を動かす・写す

同じ編集の画面の中で、選んだ文字列を、マウスで別の場所へ動かせます。

1. 文字列を選びます。
2. 選んだ範囲の上でマウスのボタンを押し、押したまま動かします。
3. 落とす場所に、キャレットの線が出ます。選択は、そのままです。
4. 場所を決めて、ボタンを離します。

- 何も押さずに離すと、文字列が**移動**します。移動した文字列が、そのまま選ばれた状態になります。
- `Ctrl` を押したまま離すと、文字列が**コピー**されます。コピーした文字列が選ばれます。
- 1 回の移動・コピーは、取り消し（`Ctrl+Z`）で、1 つの手順として戻ります。
- 選んだ範囲の上に落としたときは、何も変わりません。
- 見えている画面の外（窓の外）で離したときも、何も変わりません。
- 動かさずに離したときは、ただのクリックです。選択が解かれ、そこにキャレットが動きます。
- 矩形で選んだ範囲も、同じように動かせます。

共通設定の「編集」の「ドラッグ && ドロップ」で、次の 2 つを選べます。

| 項目 | 内容 |
|---|---|
| ドラッグ&ドロップ編集する | オフにすると、選択範囲の上で押してドラッグしたとき、文字列は動かさずに、選択の範囲を変えます |
| ドロップ元にする | オフにしたときも、同じ動きです |

## ファイルを落として開く

エクスプローラーなどからファイルをウィンドウの上へドラッグして、離すと、そのファイルを開きます。落とせる場所は、ウィンドウのどこでも構いません。

- 一度に開くファイルの数には、上限があります。既定は 8 個で、共通設定の「ファイル」の「ファイルのドロップは一度に MAX」で 1 から 99 まで変えられます。上限を超えた分は、開きません。
- すでに開いているファイルは、新しく開かずに、そのタブ（ウィンドウ）を前に出します。
- そうでないファイルは、このウィンドウのタブや、新しいウィンドウに開きます。どちらになるかは、ファイルを開くときと同じ規則です（「[複数の文書：タブとウィンドウ](tabs-windows.md)」を見てください）。
- 共通設定の「ファイル」の「ファイルのドロップは『閉じて開く』にする」をオンにすると、動きが変わります。先頭の 1 個のファイルだけを取り、今の文書を閉じて、そこに開きます。文書に未保存の変更があるときは、先に保存するかどうかを聞きます。

## サクラエディタとの違い

次のものは、まだできません。

- ウィンドウをまたぐ、文字列のドラッグ&ドロップ。
- ほかのアプリへ文字列をドラッグすることと、ほかのアプリの文字列をドロップすること。
- 右ボタンでドラッグして落としたときの、メニュー。
- ファイルのドロップでの右ボタンのメニューと、編集ウィンドウの数の上限。

理由は、GPUI が、ウィンドウをまたぐドラッグや、押したボタンの情報を渡さないためです（`docs/sakura-gaps.md`）。

## 根拠

- マツエディタのコード: `crates/matsu-ui/src/view/mouse.rs`（ドラッグの始まりと終わり）、`crates/matsu-commands/src/context/mouse_drop.rs`（同じ画面での移動・コピー）、`crates/matsu-ui/src/file_ops/drop_zone.rs`（ファイルのドロップ）、`crates/matsu-ui/src/file_ops/load.rs`（`drop_files`、`DropSettings`）、`crates/matsu-commands/src/edit_options.rs`（`drag_drop`、`drop_source`の既定）
- 設定の画面: `crates/matsu-ui/src/common_settings/pages/edit.rs`（ドラッグ && ドロップ）、`crates/matsu-ui/src/common_settings/pages/file.rs`（ファイルのドロップ）
- 試験: `crates/matsu-ui/src/view/mouse_tests.rs` の `dragging_the_selection_moves_it_as_one_undo_step`、`dragging_with_ctrl_copies`、`a_press_in_the_selection_without_a_drag_is_a_click`、`text_let_go_outside_the_view_is_not_moved`、`dropping_on_the_selection_changes_nothing`、`without_drag_and_drop_a_press_in_the_selection_selects_again`、`dragging_a_box_moves_it`
- サクラのソース: `sakura_core/view/CEditView_Mouse.cpp`（`Drop`、`DragSelection`、`TranslateDropEffect`、`OnMyDropFiles`）、`sakura_core/window/CEditWnd.cpp`（`OnDropFiles`）
- 差の記録: `docs/sakura-gaps.md`（「マウスの残り」、「ファイルのドロップ: 右ボタンのメニュー、編集ウィンドウの上限」の行）
