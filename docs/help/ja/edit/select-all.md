# すべて選択

文書全体を選びます。

## 呼び出し方

- メニュー: 「編集」→「すべて選択」（「選択」の副メニューにも同じ項目があります）
- キー: `Ctrl+A`。macOS では `Cmd+A` も使えます
- マウス: クアドラプルクリック（4 回続けてクリック）
- マクロ: `SelectAll()`
- 機能名: `F_SELECTALL`

## 動き

- 文書の先頭から終わりまでを選びます。最後が改行で終わる文書では、その後ろの空の行までが範囲です。
- カーソルは、選択の終わり（文書の終わり）に行きます。

## サクラエディタとの違い

サクラエディタは、選択の範囲だけを決め、カーソルを元の位置に残します。MATSU Editor では、カーソルが選択の一端なので、カーソルが文書の終わりへ動き、選択の始まりが文書の先頭になります。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Select.cpp` の `Command_SELECTALL`
- MATSU のコード: `crates/matsu-commands/src/commands/select/select_all.rs`
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`A`、`クアドラプルクリック` の行）
