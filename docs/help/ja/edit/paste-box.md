# 矩形貼り付け

クリップボードの文字を、矩形（四角い範囲）として貼り付けます。

## 呼び出し方

- メニュー: 「編集」→「矩形貼り付け」
- キー: `Shift+F9`
- マクロ: `PasteBox(option)`
- 機能名: `F_PASTEBOX`

## 動き

- 貼る文字の 1 行目をカーソルの位置に、2 行目をその下の行の同じ表示桁に、というように入れていきます。
- 短い行は、空白で桁をそろえてから入れます。
- 矩形選択でコピーした文字でなくても、矩形として入れます。
- クリップボードに文字がないときは、警告音が鳴ります。

## 関連

[貼り付け](paste.md)、[矩形選択](box-select.md)

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Clipboard.cpp` の `Command_PASTEBOX`
- MATSU のコード: `crates/matsu-commands/src/commands/clipboard/pastebox.rs`（`CommandContext::paste_box` を呼ぶ）
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F9` の行）
