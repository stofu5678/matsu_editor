# 矩形選択と矩形の編集

矩形選択は、2 つの桁と 2 つの行ではさんだ、四角い範囲を選ぶ方法です。表の 1 列だけを選んで消したり、縦に並んだ行の頭に同じ文字を入れたりするときに使います。

## 矩形で選ぶ

| 方法 | 動き |
|---|---|
| `Alt` を押しながら左ドラッグ | ドラッグした 2 点を角とする四角を選ぶ |
| `Alt` + 動くキー（`←` `→` `↑` `↓`、`Home` `End`、`PgUp` `PgDn`） | キャレットを動かして、四角を広げる |
| `Shift+F6`（`F_BEGIN_BOX`） | 矩形選択を始める（キャレットの位置から新しい四角を始める。それまでの選択は捨てられる）。そのあとの動くキーが、`Alt` なしで四角を広げる |

macOS では `Alt` は `Option` キーです。

- 四角の幅は、画面の桁（半角が 1、全角が 2、Tab は表示の幅）で数えます。全角や Tab をまたぐ四角は、その文字の桁に合わせて選ばれます。
- 四角の角は、行の終わりより右の、文字のない場所にも置けます。短い行は、その部分が空として扱われます。
- 押した位置と同じ場所まで戻してボタンを離すと、四角は空になり、矩形選択はやめになります。
- 矩形のまま `Shift` を押しながらクリックすると、矩形は解かれて、通常の範囲選択になります。
- 選んだ四角の上で押してドラッグすると、四角を別の場所へ動かせます。
- 選んでいる間、ステータスバーのメッセージ欄に「幅の桁数、幅のピクセル数、行数」が出ます（「[ステータスバー](status-bar.md)」を見てください）。

## 矩形のコピー・切り取り・削除

- コピーすると、四角の中の各行の文字が、1 行ずつの文字列になってクリップボードに入ります。どの行も、最後に CRLF が付きます（文書の改行コードが LF でも CRLF です）。行の改行そのものは、コピーされません。四角より短い行や、文字のない行は、空の行になります。クリップボードには、矩形であることの印も付きます。
- 切り取りは、コピーしてから四角の中を削除します。1 回の取り消しで戻ります。
- `Del`、`BkSp` は、四角の中の文字を削除します。行の終わりの改行は残ります。短い行は、四角にかかっている分だけが消えます。
- 「単語の左端まで削除」「単語の右端まで削除」は、矩形選択中は音が鳴るだけで、何もしません。

## 矩形に入力する

四角を選んだまま文字を打つと、四角の左端（その桁）に、各行同じ文字が入ります。入った分だけ四角が右へ動き、続けて打てます。

- 四角より短い行には、桁までの空白を足してから入ります。
- 全角文字を打つと、四角は 2 桁動きます。
- Tab を打つと、四角は、各行で Tab が進める桁数のうち、一番小さいものだけ動きます。
- 打った文字は、1 回の取り消しで戻り、取り消すと四角も元に戻ります。
- 上書きモードでも、四角の中は消えません。共通設定の「編集」の「矩形入力で選択範囲を削除する」をオンにすると、先に四角の中を消してから入力します（既定はオフ）。
- `Enter` は、四角の中を消して、その位置で改行します。
- 貼り付け（`Ctrl+V`）をすると、貼る文字列の最初の行が、各行の四角の左に入ります。

字下げ（`Tab`、`Space`）は、四角の左端に、その桁で文字のある行にだけ入ります。四角の幅が 0 のときは、1 回目の押下で四角を右端まで広げるだけで、字下げしません。矩形のまま字下げを戻すことは、サクラエディタと同じく、できません。

## 矩形として貼り付ける

「矩形貼り付け」（`Shift+F9`、`F_PASTEBOX`）は、クリップボードの文字列を、行ごとに、キャレットの桁に縦に並べて入れます。

- 1 行目はキャレットの行、2 行目は次の行、と順に入ります。
- 行が足りないときは、行が増えます。短い行は、桁まで空白で埋めます。
- 通常の貼り付け（`Ctrl+V`）でも、矩形としてコピーしたものは、矩形として貼られます（サクラエディタの設定 `bAutoColumnPaste` に当たる動きで、既定はオンです。マツエディタには、この設定の画面はありません）。

## 矩形に対する変換

変換の機能（大文字・小文字、全角・半角、Tab と空白の変換、行頭と行末の空白の削除など）は、矩形を選んでいると、四角の中の文字にだけ働きます。四角が文字を含まないときは、何もしません。「マージ」（`F_MERGE`）は、矩形では何もしません。

## サクラエディタとの違い

- 矩形のコピーでは、サクラエディタが行の終わりの 1 文字を削る処理をしますが、マツエディタは、各行を改行なしの文字だけで扱ってから、CRLF を付けます。そのため、CRLF の文書でも、行の終わりに CR が残りません。
- 共通設定の「編集」の「矩形選択移動で選択をロックする」は、灰色で使えません。
- 矩形のとき、折り返した画面での矩形ソートの桁が、サクラエディタと違うことがあります（通常の使い方では関係しません）。

## 根拠

- マツエディタのコード: `crates/matsu-commands/src/context/boxsel.rs`（矩形の選択と移動）、`crates/matsu-commands/src/context/box_edit.rs`（コピー、削除、入力、字下げ、変換、貼り付け）、`crates/matsu-commands/src/commands/box_select/`（`_BOX` の機能）、`crates/matsu-commands/src/commands/clipboard/paste.rs`、`crates/matsu-commands/src/commands/clipboard/pastebox.rs`、`crates/matsu-ui/src/view/mouse.rs`
- 試験: `crates/matsu-commands/src/context/box_edit_tests.rs` の `copy_gives_the_text_of_each_row_with_the_box_mark`、`copy_never_takes_a_line_ending`、`delete_and_backspace_delete_the_box`、`typing_puts_the_char_before_the_box_in_every_row`、`typing_pads_a_row_that_is_shorter_than_the_box`、`typing_in_overwrite_mode_does_not_delete_the_box`、`enter_deletes_the_box_and_breaks_the_line`、`inserting_text_puts_its_first_line_before_the_box_in_every_row`、`pastebox_pads_short_rows_with_spaces_and_adds_rows`。`crates/matsu-ui/src/view/mouse_tests.rs` の `alt_drag_selects_a_box_and_the_release_of_an_empty_one_ends_it`、`shift_click_extends_a_box_from_the_caret`、`dragging_a_box_moves_it`
- サクラのソース: `sakura_core/cmd/CViewCommander_Select.cpp`（`Command_BEGIN_BOXSELECT`）、`sakura_core/cmd/CViewCommander_Clipboard.cpp`（`Command_PASTEBOX`）、`sakura_core/cmd/CViewCommander_Edit_advanced.cpp`（`Command_INDENT`）、`sakura_core/view/CEditView.cpp`（`GetSelectedData`、`DeleteData`）
- 差の記録: `docs/sakura-gaps.md`（「共通設定『編集』の灰色 2 個」、「折り返し表示のときの矩形ソートの桁」の行）
