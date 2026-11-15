# カスタムメニューの関数

右クリックメニュー（カスタムメニュー 0）と、カスタムメニュー 1 から 24 を出す関数です。メニューの中身は、共通設定の「カスタムメニュー」のページで決めます。

## 表

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `RMenu` | なし | なし | 右クリックメニュー（カスタムメニュー 0）を、ポップアップで出す。選んだ項目の関数は、メインメニューの項目と同じやり方で実行する。 | しない | 選んだ関数は、キーマクロに記録される。メニューを出す関数そのものは、記録されない。 |
| `CustMenu1` から `CustMenu24` | なし | なし | カスタムメニュー（番号は 1〜24）を、ポップアップで出す。選んだ項目の関数は、メインメニューの項目と同じやり方で実行する。 | しない | 選んだ関数は、キーマクロに記録される。メニューを出す関数そのものは、記録されない。 |


## 根拠

- 表の元: `crates/matsu-commands/src/sakura/macro_functions.rs`（`RMenu`、`CustMenu1` から `CustMenu24`）
- サクラ: `sakura_core/cmd/CViewCommander_CustMenu.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/custom_menu/`（`menu_rbutton.rs`、`custmenu_1.rs` から `custmenu_24.rs`）、`crates/matsu-ui/src/custom_menu/mod.rs`
