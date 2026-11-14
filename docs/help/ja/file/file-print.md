# 印刷

文書を印刷します。

- 機能名: `F_PRINT`
- メニュー: 「ファイル」の「印刷...」
- 既定のキー: `Ctrl+P`
- マクロ: `Print()`

## 動き

1. [印刷プレビュー](file-print-preview.md)を開きます
2. OS の印刷ダイアログを出します。プリンターの選択や、部数などは、このダイアログで決めます
3. 印刷したら、プレビューを閉じて、文書に戻ります。ダイアログをキャンセルしたときは、プレビューが残ります

印刷するページがないとき（文書が空など）は、プレビューに「印刷するページがありません。」と出ます。

ページの大きさ、余白、フォントなどの設定は、[印刷ページ設定](file-print-pagesetup.md)で決めます。

## サクラエディタとの違い

プリンターの選択は、OS の印刷ダイアログに任せています。サクラエディタの印刷ページ設定にあるプリンターの項目はありません（`docs/sakura-gaps.md` の「印刷」）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_PRINT`、`sakura_core/print/CPrintPreview.cpp` の `OnPrint`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/print.rs`、`crates/matsu-ui/src/print/mod.rs` の `print`・`print_from_preview`、`crates/matsu-ui/src/print/preview.rs`（「印刷するページがありません。」）
- 差の記録: `docs/sakura-gaps.md`
- 設計: `docs/adr/0011-printing.md`（ADR-0011）
