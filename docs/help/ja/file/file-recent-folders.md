# 最近使ったフォルダー

最近使ったフォルダーの一覧から、そのフォルダーで「開く」のダイアログを出します。

- 機能名: `F_FOLDER_USED_RECENTLY`（メニューの中に一覧を作るための印で、単独のコマンドではありません）
- メニュー: 「ファイル」の「最近使ったフォルダー」の子のメニュー
- 既定のキー: なし
- マクロ: なし

## 動き

子のメニューに、最近のフォルダーが 1 行ずつ並びます。[最近使ったファイル](file-recent-files.md)と同じ形で、行の頭の番号、お気に入りの星（★）、「ファイル名表示」の設定に従ったパスが出ます。文字コードは付きません。

行を選ぶと、そのフォルダーから始まる「開く」のダイアログが出ます。選んだファイルは、[開く](file-open.md)と同じ方法で開きます。ダイアログをキャンセルすると、何も開きません。

履歴の数の上限は 36 です。

## サクラエディタとの違い

[最近使ったファイル](file-recent-files.md)と同じです。

## 根拠

- サクラのソース: `sakura_core/recent/CMRUFolder.cpp` の `CreateMenu`、`sakura_core/window/CEditWnd.cpp` の `OnCommand`（`IDM_SELOPENFOLDER`）
- MATSU のコード: `crates/matsu-ui/src/history/menu.rs`（`folder_label`）、`crates/matsu-ui/src/history/view.rs` の `open_recent_folder`、`crates/matsu-config/src/history/mod.rs`（`MAX_OPENFOLDER`）
