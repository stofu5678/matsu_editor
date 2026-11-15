# やり直し

「元に戻す」で取り消した編集を、もう一度行います。

## 呼び出し方

- メニュー: 「編集」→「やり直し」
- キー: `Ctrl+Y`。macOS では `Cmd+Shift+Z` も使えます
- マクロ: `Redo()`
- 機能名: `F_REDO`

## 動き

- 直前の「元に戻す」が取り消したまとまりを、もう一度適用します。
- カーソルは、やり直した操作が終わったときの位置に行きます。
- やり直せるものがないときは、何も起きません。
- 取り消したあとで新しく編集すると、やり直せる内容は消えます。

## 関連

[元に戻す](undo.md)

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Edit.cpp` の `CViewCommander::Command_REDO`、`sakura_core/cmd/COpeBuf.cpp` の `COpeBuf::DoRedo`
- MATSU のコード: `crates/matsu-commands/src/commands/edit/redo.rs`
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`Y` の行）、`docs/adr/0008-commands-and-keybindings.md`
