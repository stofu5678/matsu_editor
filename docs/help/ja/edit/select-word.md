# 現在位置の単語選択

カーソルのある単語を選びます。

## 呼び出し方

- メニュー: 「編集」→「高度な操作」→「現在位置の単語選択」（「選択」の副メニューにも同じ項目があります）
- キー: `Ctrl+W`
- マウス: ダブルクリック（Shift、Ctrl、Alt を押していても同じ）
- マクロ: `SelectWord()`
- 機能名: `F_SELECTWORD`

## 動き

- もとの選択は、最初に外されます。
- 単語は、カーソルの位置にある、同じ種類の文字が続くひとまとまりです。空白の上にカーソルがあれば、続く空白が選ばれます。
- 改行の上、行の終わりより右、最後に改行がある文書のその後ろの空の行には、単語がありません。何も選ばれず、警告音も鳴りません。
- 選択の始まりが単語の先頭、カーソルが単語の終わりになります。

## サクラエディタとの違い

サクラエディタは選択の範囲を決めてカーソルをその終わりに置きます。MATSU Editor では、カーソルが選択の一端なので、結果は同じように見えます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Select.cpp` の `Command_SELECTWORD`、`sakura_core/agent/CSearchAgent.cpp` の `WhereCurrentWord`、`sakura_core/parse/CWordParse.cpp` の `WhereCurrentWord_2`
- MATSU のコード: `crates/matsu-commands/src/commands/select/select_word.rs`、単語の分け方は `crates/matsu-commands/src/text/word.rs`
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`ダブルクリック`、`W` の行）
