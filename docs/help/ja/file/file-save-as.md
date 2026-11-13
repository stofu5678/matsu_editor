# 名前を付けて保存

保存先の名前を選んで、保存します。

- 機能名: `F_FILESAVEAS_DIALOG`
- メニュー: 「ファイル」の「名前を付けて保存...」
- 既定のキー: `Ctrl+Shift+S`
- マクロ: `FileSaveAsDialog(ファイル名, 文字コード, 改行コード)`

## 動き

1. 保存のダイアログが出ます。今のファイルのフォルダーから始まり、今のファイルの名前を提案します。ファイルがない文書のときは、「無題_日付_時刻」の形の名前を提案します
2. 名前を決めると、保存します。保存の途中の確認は、[上書き保存](file-save.md)と同じです

ダイアログには、文字コード、BOM、改行コードを選ぶ欄があります。ダイアログの作りの都合でその欄を出せないとき（GPUI のダイアログ）は、名前を決めたあとに、小さなダイアログで同じことを聞きます。

## マクロの引数

| 引数 | 意味 |
|---|---|
| 1 つ目 | 提案するファイル名（まだ使っていません） |
| 2 つ目 | 文字コードの番号。99 か省略で、今のまま |
| 3 つ目 | 改行コード。0 か省略で、変えない。1 が CRLF、2 が LF、3 が CR |

2 つ目か 3 つ目を渡すと、文字コードと改行コードを聞くダイアログは出ません。

## サクラエディタとの違い

- 1 つ目の引数（提案するファイル名）は、まだ効きません
- Windows の旧式のファイルダイアログは、ありません（新式だけ）

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_FILESAVEAS_DIALOG`、`sakura_core/doc/CDocFileOperation.cpp` の `FileSaveAs`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/file_save_as_dialog.rs`、`crates/matsu-ui/src/file_ops/save.rs` の `choose_path`、`crates/matsu-ui/src/file_ops/save_as_dialog.rs`
- 差の記録: `docs/sakura-gaps.md`
