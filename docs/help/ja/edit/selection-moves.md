# 選択の移動

「編集」→「選択」の副メニューにある、「(選択)」で始まる機能です。[移動](cursor-moves.md)と同じ場所へ動き、動いた分だけ選択を広げます。

選択の始まりは、動く前のカーソル位置です。選択がすでにあるときは、その始まりをそのままにして、カーソルの側を動かします。

「範囲選択開始」「現在位置の単語選択」「すべて選択」も同じ副メニューにあります（[範囲選択開始](begin-select.md)、[現在位置の単語選択](select-word.md)、[すべて選択](select-all.md)）。

## 一覧

| 機能名 | メニュー | キー | マクロ | 広げる動き |
|---|---|---|---|---|
| `F_UP2_SEL` | (選択)カーソル上移動(２行ごと) | `Ctrl+Shift+↑` | `Up2_Sel()` | 2 行上 |
| `F_DOWN2_SEL` | (選択)カーソル下移動(２行ごと) | `Ctrl+Shift+↓` | `Down2_Sel()` | 2 行下 |
| `F_WORDLEFT_SEL` | (選択)単語の左端に移動 | `Ctrl+Shift+←` | `WordLeft_Sel()` | 単語の左端 |
| `F_WORDRIGHT_SEL` | (選択)単語の右端に移動 | `Ctrl+Shift+→` | `WordRight_Sel()` | 単語の右端 |
| `F_GOPREVPARAGRAPH_SEL` | (選択)前の段落へ移動 | なし | `GoPrevParagraph_Sel()` | 前の段落 |
| `F_GONEXTPARAGRAPH_SEL` | (選択)次の段落へ移動 | なし | `GoNextParagraph_Sel()` | 次の段落 |
| `F_GOLINETOP_SEL` | (選択)行頭に移動(折り返し単位) | `Shift+Home` | `GoLineTop_Sel(フラグ)` | 行頭 |
| `F_GOLINEEND_SEL` | (選択)行末に移動(折り返し単位) | `Shift+End` | `GoLineEnd_Sel()` | 行末 |
| `F_1PageUp_Sel` | (選択)１ページアップ | `Shift+PageUp` | `PageUp_Sel(行数)`、`1PageUp_Sel(行数)` | 1 ページ上 |
| `F_1PageDown_Sel` | (選択)１ページダウン | `Shift+PageDown` | `PageDown_Sel(行数)`、`1PageDown_Sel(行数)` | 1 ページ下 |
| `F_GOFILETOP_SEL` | (選択)ファイルの先頭に移動 | `Ctrl+Shift+Home` | `GoFileTop_Sel()` | 文書の先頭 |
| `F_GOFILEEND_SEL` | (選択)ファイルの最後に移動 | `Ctrl+Shift+End` | `GoFileEnd_Sel()` | 文書の終わり |
| `F_MODIFYLINE_NEXT_SEL` | (選択)次の変更行へ | なし | `GoModifyLineNext_Sel()` | 次の変更行 |
| `F_MODIFYLINE_PREV_SEL` | (選択)前の変更行へ | なし | `GoModifyLinePrev_Sel()` | 前の変更行 |

各機能の動き（どこへ動くか）は、[移動](cursor-moves.md)と[関数リストマークと変更行への移動](marks.md)にあります。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Cursor.cpp`（`bSelect` を付けた `Command_UP2`、`Command_WORDLEFT` など）、`sakura_core/cmd/CViewCommander.cpp` の `HandleCommand`
- MATSU のコード: `crates/matsu-commands/src/commands/cursor/` の `*_sel.rs`（`cursor_up2_sel.rs`、`word_left_sel.rs`、`goprevparagraph_sel.rs`、`go_line_top_sel.rs`、`go_file_top_sel.rs` など）。`crates/matsu-ui/src/ui_commands/cursor/one_page_up_sel.rs`、`one_page_down_sel.rs`、`crates/matsu-ui/src/ui_commands/select/modifyline_next_sel.rs`、`modifyline_prev_sel.rs`
- 表の対応: `crates/matsu-commands/src/sakura/macro_functions.rs`（マクロ名）、`key_bindings.rs`（キー）
