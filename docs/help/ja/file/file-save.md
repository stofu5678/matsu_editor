# 上書き保存

文書を、今のファイルに保存します。

- 機能名: `F_FILESAVE`
- メニュー: 「ファイル」の「上書き保存」
- 既定のキー: `Ctrl+S`
- マクロ: `FileSave()`

## 動き

1. ファイルがない文書（無題）のときは、保存先を聞くダイアログが出ます。[名前を付けて保存](file-save-as.md)と同じです
2. ディスクにあるファイルで、変更がない文書は、何も書きません。「変更がないので保存しなかった」と出ます。名前はあるのにディスクにファイルがない文書（開こうとして、なかったファイルなど）は、変更がなくても、ファイルを作るために書きます
3. 改行コードが混在しているときは、「現在の入力改行コードに統一しますか」と聞きます
4. 今の文字コードで書けない文字があるときは、最初の場所を見せて、続けるか聞きます。続けると、その文字は `?` になります
5. 共通設定でバックアップを作る設定のときは、書く前にバックアップを作ります
6. 書きます。失敗したときは理由を出し、元のファイルはそのままです

ビューモードで開いている文書は、同じファイルには保存できません。「ビューモードでは同一ファイルへの保存はできません。」と出ます。別の名前での保存はできます。

### 設定で変わること

| 設定 | 動き |
|---|---|
| 共通設定「ファイル」の「無変更でも上書きする」 | 入れると、変更がなくても書きます（既定は切） |
| タイプ別設定「支援」の「保存時に改行コードの混在を警告する」 | 切にすると、手順 3 の確認を出しません |

## マクロ `FileSaveAs`

ダイアログを出さずに、名前を指定して保存するマクロの関数です（機能名 `F_FILESAVEAS`）。

| 引数 | 意味 |
|---|---|
| 1 つ目 | 保存するファイル（必須） |
| 2 つ目 | 文字コードの番号。99 か省略で、今のまま |
| 3 つ目 | 改行コード。0 か省略で、変えない。1 が CRLF、2 が LF、3 が CR |

マクロは質問に答えられないので、改行コードの混在の確認と、書けない文字の確認は出しません。混在した行はそのまま書き、書けない文字は `?` にします。

文字コードを変えて保存したときの BOM は、変える前の文字コードの既定に従います。たとえば UTF-16 のファイルを UTF-8 で保存すると、BOM が付きます。サクラエディタの実機で同じ結果になることを確かめています。

## サクラエディタとの違い

- 動きはサクラエディタと同じです
- 保存の途中の進み具合とキャンセルは、まだありません（`docs/sakura-gaps.md`）

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_FILESAVE`、`sakura_core/doc/CDocFileOperation.cpp` の `FileSave`・`DoSaveFlow`、`sakura_core/env/CCodeChecker.cpp` の `OnCheckSave`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/file_save.rs`、`crates/matsu-ui/src/ui_commands/file/file_save_as.rs`、`crates/matsu-ui/src/file_ops/save.rs`（先頭のコメントに手順がある）、`crates/matsu-ui/src/file_ops/ask.rs`（確認の文）
- 設定の名前: `crates/matsu-ui/src/common_settings/pages/file.rs`、`crates/matsu-ui/src/type_settings/pages/support.rs`
- サクラで確かめた結果: `crates/matsu-ui/src/ui_commands/file/file_save_as.rs` のコメントにある `save-utf16le-to-utf8`、`save-sjis-to-utf16le`、`save-mixed-eol-keeps`
