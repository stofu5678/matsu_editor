# キー割り当て一覧をコピー

いまのキー割り当てを、表の形の文字にして、クリップボードに入れます。

- 機能名: `F_CREATEKEYBINDLIST`
- メニュー: 「ヘルプ」→「キー割り当て一覧をコピー」
- 既定のキー: `Ctrl+Q`（macOS では `Control+Q`）
- マクロ: `CopyKeyBindList()`（引数なし。キーマクロに記録されます）

## できる文字

タブで区切った表です。行の終わりは、CR と LF です。

1. 1 行目は見出しです: 「キー」「機能名」「関数名」「機能番号」「キーマクロ記録可/不可」
2. 2 行目は、区切りの `-----` です
3. 3 行目からは、機能が割り当てられたキーごとに 1 行です

| 列 | 中身 |
|---|---|
| キー | `Ctrl+C` のような、キーの名前。修飾キーは `Shift+` `Ctrl+` `Alt+` の順に付く |
| 機能名 | 「共通設定」の「キー割り当て」ページに出る、機能の名前。名前がないときは `---名前が定義されていない-----` |
| 関数名 | マクロで呼ぶときの名前。ないときは空 |
| 機能番号 | 機能のコード（`F_COPY` なら 30602） |
| キーマクロ記録可/不可 | キーマクロに記録される機能なら `○`、記録されないなら `×` |

- 行の並びは、修飾キーの組み合わせの順です。まず何も押さない行、次に `Shift`、`Ctrl`、`Shift+Ctrl`、`Alt`、`Shift+Alt`、`Ctrl+Alt`、`Shift+Ctrl+Alt` です。それぞれの中は、キーの表の順です。
- 機能が割り当てられていないキーの組み合わせは、行になりません。
- 使うのは、いまのキー割り当て（「共通設定」で変えたあとの表）です。標準の表ではありません。

文書には何も書きません。貼り付けると、表が文書に入ります。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Clipboard.cpp` の `Command_CREATEKEYBINDLIST`、`sakura_core/func/CKeyBind.cpp` の `CKeyBind::CreateKeyBindList`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/clipboard/createkeybindlist.rs`、`crates/matsu-commands/src/keymap/bind_list.rs`（`create_key_bind_list`）
- 試験: `crates/matsu-commands/src/keymap/bind_list.rs` の `the_list_has_the_header_and_a_line_for_each_function_of_a_key`、`the_modifiers_go_in_the_order_shift_ctrl_alt_and_the_combinations_in_turn`、`the_key_macro_column_follows_the_table`
- 既定のキーとマクロ名: `crates/matsu-commands/src/sakura/key_bindings.rs`（`Q` の行）、`crates/matsu-commands/src/sakura/macro_functions.rs`（`CopyKeyBindList`）
