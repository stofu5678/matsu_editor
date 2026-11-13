# コマンド: ファイル

「ファイル」メニューの機能を説明する章です。ファイルを作る、開く、保存する、閉じる、開き直す、印刷する、終了する機能が入っています。

## 機能の一覧

メニューの並びの順です。

| メニューの項目 | 機能名 | 既定のキー | ページ |
|---|---|---|---|
| 新規作成 | `F_FILENEW` | `Ctrl+N` | [新規作成](file-new.md) |
| 新規ウインドウを開く | `F_FILENEW_NEWWINDOW` | なし | [新規ウインドウを開く](file-new-newwindow.md) |
| 開く... | `F_FILEOPEN` | `Ctrl+O` | [開く](file-open.md) |
| 上書き保存 | `F_FILESAVE` | `Ctrl+S` | [上書き保存](file-save.md) |
| 名前を付けて保存... | `F_FILESAVEAS_DIALOG` | `Ctrl+Shift+S` | [名前を付けて保存](file-save-as.md) |
| すべて上書き保存 | `F_FILESAVEALL` | なし | [すべて上書き保存](file-save-all.md) |
| 保存して閉じる | `F_FILESAVECLOSE` | なし | [保存して閉じる](file-save-close.md) |
| 閉じる | `F_WINCLOSE` | なし | [閉じる](file-win-close.md) |
| 閉じて(無題) | `F_FILECLOSE` | なし | [閉じて(無題)](file-close.md) |
| 閉じて開く... | `F_FILECLOSE_OPEN` | `Ctrl+Shift+F4` | [閉じて開く](file-close-open.md) |
| 開き直す（子のメニュー） | `F_FILE_REOPEN` と、文字コードごとの 9 個 | 文字コードごとに `Ctrl+Alt+F7` から `Ctrl+Alt+F12` など | [開き直す](file-reopen.md) |
| 印刷... | `F_PRINT` | `Ctrl+P` | [印刷](file-print.md) |
| 印刷プレビュー | `F_PRINT_PREVIEW` | `Ctrl+Shift+P` | [印刷プレビュー](file-print-preview.md) |
| 印刷ページ設定... | `F_PRINT_PAGESETUP` | `Ctrl+Alt+P` | [印刷ページ設定](file-print-pagesetup.md) |
| ファイルのプロパティ | `F_PROPERTY_FILE` | `Alt+Enter` | [ファイルのプロパティ](file-property.md) |
| ブラウズ | `F_BROWSE` | `Ctrl+B` | [ブラウズ](file-browse.md) |
| 最近使ったファイル | `F_FILE_USED_RECENTLY`（メニューの一覧） | なし | [最近使ったファイル](file-recent-files.md) |
| 最近使ったフォルダー | `F_FOLDER_USED_RECENTLY`（メニューの一覧） | なし | [最近使ったフォルダー](file-recent-folders.md) |
| グループを閉じる | `F_GROUPCLOSE` | なし | [グループを閉じる](file-groupclose.md) |
| 編集の全終了 | `F_EXITALLEDITORS` | `Alt+Shift+F4` | [編集の全終了](file-exit-all-editors.md) |
| マツエディタの全終了 | `F_EXITALL` | `Ctrl+Alt+F4` | [マツエディタの全終了](file-exit-all.md) |

- 既定のキーは、サクラエディタの既定の割り当てです。キー割り当てを変えていると、違うことがあります
- 機能名はサクラエディタと同じです。マクロから呼ぶ名前は、各ページに書いています
- 「開き直す」の子のメニューは、動きが同じなので 1 ページにまとめています
- マクロからだけ使える `FileSaveAs`（ダイアログなしで保存）と `FileOpen`（ファイルを指定して開く）は、[上書き保存](file-save.md)と[開く](file-open.md)の「マクロ」の節に書いています

## 未保存の変更がある文書を閉じるとき

閉じる機能の多くは、文書に変更があると、先に「保存しますか」と聞きます。答えは次の 3 つです。

- 「保存する(Y)」: 保存してから閉じます。保存が途中でやめになったら、閉じません
- 「保存しない(N)」: 変更を捨てて閉じます
- 「キャンセル」: 何も閉じません

Grep を実行している間は、閉じられません。ウィンドウに「Grepの処理中です。」と出ます。

## 根拠

- サクラのソース: `sakura_core/Funccode_x.hsrc`、`sakura_core/func/CKeyBind.cpp`、`sakura_core/doc/CEditDoc.cpp` の `OnFileClose`
- MATSU のコード: `crates/matsu-commands/src/sakura/main_menu.rs`、`crates/matsu-commands/src/sakura/functions.rs`、`crates/matsu-commands/src/sakura/key_bindings.rs`、`crates/matsu-ui/src/file_ops/close.rs` の `ask_about_changes`
