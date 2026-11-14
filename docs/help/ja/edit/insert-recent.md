# 最近使ったファイル挿入と最近使ったフォルダー挿入

最近使ったファイル（またはフォルダー）のパスを、文書に書き出します。

| 機能名 | メニュー | マクロ | 入るもの |
|---|---|---|---|
| `F_INS_FILE_USED_RECENTLY` | 「編集」→「挿入」→「最近使ったファイル挿入」 | `InsertFileUsedRecently()` | 最近使ったファイルのパス |
| `F_INS_FOLDER_USED_RECENTLY` | 「編集」→「挿入」→「最近使ったフォルダー挿入」 | `InsertFolderUsedRecently()` | 最近使ったフォルダーのパス |

どちらも、既定のキーの割り当てはありません。

## 動き

- 「ファイル」メニューの「最近使ったファイル」（または「最近使ったフォルダー」）に出る一覧を、新しいものから順に、1 行に 1 つずつ書きます。各行の終わりには、文書の改行コードが付きます。
- カーソルの位置に入れます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Insert.cpp` の `Command_INS_FILE_USED_RECENTLY`、`Command_INS_FOLDER_USED_RECENTLY`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/history/ins_file_used_recently.rs`、`ins_folder_used_recently.rs`
