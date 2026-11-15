# 印刷プレビュー

印刷したときのページの見た目を、画面で確かめます。

- 機能名: `F_PRINT_PREVIEW`
- メニュー: 「ファイル」の「印刷プレビュー」
- 既定のキー: `Ctrl+Shift+P`
- マクロ: `PrintPreview()`

## 動き

プレビューは、ウィンドウの中で、文書の代わりに出ます。上にボタンの並んだバーがあります。

| ボタン | 動き |
|---|---|
| 印刷... | OS の印刷ダイアログを出して、印刷します（[印刷](file-print.md)と同じ） |
| 前 / 次 | ページを移ります |
| 縮小 / 拡大 | 表示の大きさを変えます |
| ページ設定... | [印刷ページ設定](file-print-pagesetup.md)のダイアログを出します |
| 戻る | プレビューを閉じて、文書に戻ります |

バーには、「3/10頁」のように、今のページと全体のページ数が出ます。使っている用紙も、「A4  縦」のように出ます。`Esc` でも閉じます。

プレビューは、開いた時点の文書の写しで作ります。開いている間に文書が変わっても、ページは変わりません。印刷ページ設定を変えると、ページを作り直します。

## サクラエディタとの違い

サクラエディタでは、このコマンドでプレビューを開いたり閉じたりできます。MATSU Editor では、プレビューを閉じるのは「戻る」ボタンと `Esc` で、このコマンドは開くだけです。プレビューが開いているときは、何もしません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_PRINT_PREVIEW`、`sakura_core/print/CPrintPreview.cpp`、ボタンのバーは `sakura_core/sakura_rc.rc` の `IDD_PRINTPREVIEWBAR`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/print_preview.rs`（差の説明がコメントにある）、`crates/matsu-ui/src/print/mod.rs` の `open_print_preview`、`crates/matsu-ui/src/print/preview.rs`（ボタンの名前）
- 設計: `docs/adr/0011-printing.md`（ADR-0011）
