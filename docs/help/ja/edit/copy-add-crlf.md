# 折り返し位置に改行をつけてコピー

折り返し表示で行が切れている場所に、改行を足して写します。

## 呼び出し方

- メニュー: 「編集」→「折り返し位置に改行をつけてコピー」
- キー: 既定の割り当てはありません
- マクロ: `CopyAddCRLF()`
- 機能名: `F_COPY_ADDCRLF`

## 動き

- [コピー](copy.md)と同じ操作です。ただし、折り返しで切れている行の終わりと、最後の行の終わりに、文書の改行コードの改行を付けて写します。
- 文書の中は変わりません。
- 矩形選択のコピーには、この改行は付きません。

共通設定「編集」の「折り返し行に改行を付けてコピー」を入れると、ふつうの「コピー」と「切り取り」も同じ動きになります（既定は切れています）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander.cpp` の `HandleCommand`（`Command_COPY( false, true )`）、`sakura_core/cmd/CViewCommander_Clipboard.cpp`、`sakura_core/view/CEditView.cpp` の `GetSelectedData`、`CopyCurLine`
- MATSU のコード: `crates/matsu-commands/src/commands/clipboard/copy_addcrlf.rs`
- 設定の項目名: `crates/matsu-ui/src/common_settings/pages/edit.rs`（`bAddCRLFWhenCopy`）
