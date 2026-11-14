# 元に戻す

直前の編集を取り消します。

## 呼び出し方

- メニュー: 「編集」→「元に戻す」
- キー: `Ctrl+Z`（ほかに `Ctrl+_` にも割り当てています）。macOS では `Cmd+Z` も使えます
- マクロ: `Undo()`
- 機能名: `F_UNDO`

## 動き

- 1 回の操作が 1 つのまとまり（ブロック）として記録されます。「元に戻す」は、そのまとまりを 1 つずつさかのぼって取り消します。
- 取り消すと、文字と、操作の前のカーソル位置が戻ります。「選択範囲を削除」を取り消すと、その範囲がもう一度選ばれた状態になります。
- 取り消すものがないときは、何も起きません。警告音も鳴りません。

## サクラエディタとの違い

サクラエディタは、編集を始めた位置にカーソルを置き、選択を外します。MATSU Editor は、操作の前の状態（選択を含む）にカーソルを戻します。

## 関連

[やり直し](redo.md)

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Edit.cpp` の `CViewCommander::Command_UNDO`、`sakura_core/cmd/COpeBuf.cpp` の `COpeBuf::DoUndo`
- MATSU のコード: `crates/matsu-commands/src/commands/edit/undo.rs`（先頭のコメントに違いの説明）、`crates/matsu-core/src/undo` のブロックの扱い
- 試験: `crates/matsu-commands/src/commands/edit/undo.rs` の `restores_the_text_and_the_cursor`、`steps_back_one_block_at_a_time`、`nothing_to_undo`
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`Z` の行）、`docs/adr/0008-commands-and-keybindings.md`（macOS のキー）
