# 新規ウインドウを開く

名前のない新しい文書を、新しいウィンドウに作ります。

- 機能名: `F_FILENEW_NEWWINDOW`
- メニュー: 「ファイル」の「新規ウインドウを開く」（「新規作成」の次）
- 既定のキー: なし
- マクロ名: なし（`macro_functions.rs` の表に載っていません）

## 動き

[新規作成](file-new.md)との違いは、タブを使っているときの動きだけです。

- 新規作成: 同じウィンドウの新しいタブになる
- 新規ウインドウを開く: 独立した新しいウィンドウになる

タブを使っていないときは、2 つとも同じです。

## サクラエディタとの違い

動きはサクラエディタと同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_FILENEW_NEWWINDOW`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/file_new_new_window.rs`、メニューへの足し方は `crates/matsu-config/src/settings/main_menu.rs`（`F_FILENEW_NEWWINDOW after F_FILENEW` の行）
