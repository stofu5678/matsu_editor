# 移動（カーソルの移動）

「編集」→「移動」の副メニューにある、カーソルを動かす機能です。小さな機能が多いので、1 つの表にまとめました。ジャンプや履歴に関する機能は、別のページにあります（[指定行へジャンプ](jump.md)、[移動履歴と検索開始位置](jump-history.md)、[関数リストマークと変更行](marks.md)）。

## 一覧

| 機能名 | メニュー | キー | マクロ |
|---|---|---|---|
| `F_UP2` | カーソル上移動(２行ごと) | なし | `Up2()` |
| `F_DOWN2` | カーソル下移動(２行ごと) | なし | `Down2()` |
| `F_WORDLEFT` | 単語の左端に移動 | `Ctrl+←` | `WordLeft()` |
| `F_WORDRIGHT` | 単語の右端に移動 | `Ctrl+→` | `WordRight()` |
| `F_GOPREVPARAGRAPH` | 前の段落へ移動 | なし | `GoPrevParagraph()` |
| `F_GONEXTPARAGRAPH` | 次の段落へ移動 | なし | `GoNextParagraph()` |
| `F_GOLINETOP` | 行頭に移動(折り返し単位) | `Home` | `GoLineTop(フラグ)` |
| `F_GOLINEEND` | 行末に移動(折り返し単位) | `End` | `GoLineEnd()` |
| `F_1PageUp` | １ページアップ | `PageUp` | `PageUp(行数)`、`1PageUp(行数)` |
| `F_1PageDown` | １ページダウン | `PageDown` | `PageDown(行数)`、`1PageDown(行数)` |
| `F_GOFILETOP` | ファイルの先頭に移動 | `Ctrl+Home` | `GoFileTop()` |
| `F_GOFILEEND` | ファイルの最後に移動 | `Ctrl+End` | `GoFileEnd()` |
| `F_CURLINECENTER` | カーソル行をウィンドウ中央へ | `Ctrl+H` | `CurLineCenter()` |

「前の段落へ」「次の段落へ」は、マクロの関数として呼べますが、キーマクロには記録されません。

macOS では、`Cmd+←` `Cmd+→` が行頭・行末、`Cmd+↑` `Cmd+↓` がファイルの先頭・末尾になります（Shift を足すと選択）。

## 2 行ごとの移動

- カーソルを 2 行、上（または下）に動かします。上下の移動の前に覚えた表示桁へ移ります。
- 文書の端に近いときは、動けるところまで動きます。上から 2 行目にいれば、1 行だけ上がります。

## 単語の左端・右端

- 「単語の左端に移動」は、カーソルの前にある単語の先頭へ動きます。途中の空白と TAB は飛ばします。行頭のように行き先の単語がないときは、1 文字ぶん左へ動き、前の行の終わりに行きます。
- 「単語の右端に移動」は、次の単語の先頭へ動きます。空白と TAB は飛ばします。最後の単語のときは、行の終わり（改行の前）で止まります。行末にいるときは、1 文字ぶん右へ動いて次の行の先頭に行き、そこが空白か TAB で始まっていれば、もう一度動きます。
- 選択があるときは、選択を外して、カーソルの位置から動きます。

## 段落の移動

- 「前の段落へ」は、1 つ上の行に文字があるときは、その文字の続く塊の最初の行に動きます。上の行が空（空白と TAB だけの行も空とみなします）のときは、空の行を飛び越えて、その上にある文字の塊に動きます。空の行にいるときは、上の文字の塊の最初の行に動きます。
- 「次の段落へ」は、下向きに同じ動きをします。
- 動いたあとも、表示桁は保ちます。
- 先頭の行では動きません。

## 行頭・行末

- 「行頭に移動」は、行頭にある字下げ（インデント）の後ろの、最初の文字に動きます。すでにそこにいるときは、行の先頭に動きます。繰り返すと、この 2 か所を行き来します。
- 「行末に移動」は、行の終わり（改行の前）に動きます。
- どちらも、折り返し表示のときは画面の 1 行が単位です。
- マクロの `GoLineTop(フラグ)` のフラグは、足し算です。

| フラグ | 意味 |
|---|---|
| 1 | 字下げの後ろに止まらず、いつも行の先頭に動く |
| 4 | 選択を広げる |
| 8 | 画面の 1 行ではなく、文書の行（改行まで）を単位にする |

## ページ移動

- 「１ページアップ」「１ページダウン」は、カーソルと表示を、1 ページ分より 1 行少なく動かします。
- マクロの `PageUp(行数)` などの引数は、動かす行数です。省略したとき、または 0 以下のときは、この既定の動きになります。

## ファイルの先頭・末尾

文書の先頭、または末尾に動きます。

## カーソル行をウィンドウ中央へ

カーソルのある行が、ウィンドウの真ん中に来るように、表示だけをスクロールします。カーソルは動きません。文書の終わり近くでも、その行が真ん中に来ます（終わりより下に空きができます）。

## 選択の固定との関係

「範囲選択開始」（`F_BEGIN_SEL`）で選択を固定している間は、上の移動のほとんどが、選択を広げる動きになります。くわしくは[範囲選択開始](begin-select.md)を見てください。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Cursor.cpp` の `Command_UP2`、`Command_WORDLEFT`、`Command_WORDRIGHT`、`Command_GOPREVPARAGRAPH`、`Command_GONEXTPARAGRAPH`、`Command_GOLINETOP`、`Command_GOLINEEND`、`Command_1PageUp`、`Command_1PageDown`、`Command_GOFILETOP`、`Command_GOFILEEND`、`Command_CURLINECENTER`
- MATSU のコード: `crates/matsu-commands/src/commands/cursor/` の `cursor_up2.rs`、`cursor_down2.rs`、`word_left.rs`、`word_right.rs`、`goprevparagraph.rs`、`gonextparagraph.rs`、`go_line_top.rs`、`go_line_end.rs`、`go_file_top.rs`、`go_file_end.rs`。`crates/matsu-ui/src/ui_commands/cursor/` の `one_page_up.rs`、`one_page_down.rs`、`cur_line_center.rs`。ページ移動の規則は `crates/matsu-ui/src/view/paging.rs`
- 行頭移動のフラグ: `crates/matsu-commands/src/context/motion.rs`（`LINE_TOP_ONLY`、`LINE_TOP_SELECT`、`LINE_TOP_LINE_UNIT`）
- 試験: `crates/matsu-commands/src/commands/cursor/goprevparagraph.rs`、`go_line_top.rs` などの `tests` モジュール
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`、macOS は `docs/adr/0008-commands-and-keybindings.md`
