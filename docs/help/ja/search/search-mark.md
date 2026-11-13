# 検索マークの切替え

探した文字列を画面で色づけして見せる「検索マーク」を、付けたり消したりします。

- メニュー: 検索 → 検索マークの切替え
- 既定のキー: `Ctrl+F3`
- マクロ: `SearchClearMark`（機能名 `F_SEARCH_CLEARMARK`）

## 動き

- 文字列を選んでいるとき: その文字列を探す文字列にして、文書の中の一致をすべてマークします。探す文字列は残るので、続けて `F3` で次を探せます。
- 選んでいないとき: マークを消します。
- 検索すると、マークは自動で付きます。インクリメンタルサーチは、モードを始めるときにマークを消し、文字を打って探すたびにまた付けます。
- 単語単位の検索で、探す文字列がスペースで区切った複数の単語のときは、単語ごとに検索文字列の色（色の設定にある 5 色のうち、表示にしているもの）を順番に使います。それ以外は 1 色目です。
- 1 行が 64 KiB を超える行は、マークの対象外です。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Search.cpp` の `Command_SEARCH_CLEARMARK`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/search/search_clearmark.rs`、`crates/matsu-ui/src/search/ops.rs`（`toggle_search_mark`）、`crates/matsu-ui/src/search/highlight.rs`（`MAX_HIGHLIGHT_LINE_BYTES`、`slot_of`）
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F3`）
