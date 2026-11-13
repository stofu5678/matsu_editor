# 選択範囲内色付きHTMLコピー

選択している文字を、画面の色と文字の飾りのまま、HTML としてコピーします。

| 機能名 | メニュー | マクロ | 内容 |
|---|---|---|---|
| `F_COPY_COLOR_HTML` | 「編集」→「高度な操作」→「選択範囲内色付きHTMLコピー」 | `CopyColorHtml()` | 色付きの HTML |
| `F_COPY_COLOR_HTML_LINENUMBER` | 「編集」→「高度な操作」→「選択範囲内行番号色付きHTMLコピー」 | `CopyColorHtmlWithLineNumber()` | 各行の前に行番号を付けた色付きの HTML |

どちらも、既定のキーの割り当てはありません。

## 動き

- 選んだ範囲を、`<pre>` の中に、文書のタイプの色と文字の飾り（太字など）を付けた HTML にします。
- クリップボードには、HTML のソースを文字として載せ、同時に、OS が持つ HTML の形式でも載せます。HTML を貼れるアプリには、色付きで貼れます。文字だけのアプリには、HTML のソースが貼れます。

## 残っていること

Linux では、HTML の形式でのクリップボードは使えず、文字として HTML のソースが載ります。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Clipboard.cpp` の `Command_COPY_COLOR_HTML`、`Command_COPY_COLOR_HTML_LINENUMBER`
- MATSU のコード: `crates/matsu-ui/src/copy_html/mod.rs`、`crates/matsu-ui/src/ui_commands/clipboard/copy_color_html.rs`、`copy_color_html_linenumber.rs`
- 差と残り: `docs/sakura-gaps.md` の「Linux の HTML クリップボード」の行
